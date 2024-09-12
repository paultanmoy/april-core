use std::fmt;

use anyhow::anyhow;
use base64::prelude::{BASE64_STANDARD, Engine as _};
use reqwest::{Client, StatusCode};
use serde::{
    de::{self, Visitor},
    Deserialize,
    Deserializer,
    Serialize,
};
use tracing::{debug, error, info, instrument, warn};

use super::{Error, Image, LanguageModel, Message};

#[derive(Debug, Deserialize)]
pub struct AnthropicErrorResponse {
    #[serde(rename = "type")]
    error_type: String,
    
    message: String,
}

impl AnthropicErrorResponse {
    pub fn error_type(&self) -> &str {
        &self.error_type
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AnthropicImageContent {
    #[serde(rename = "type")]
    encoding: String,

    media_type: String,
    data: String,
}

impl AnthropicImageContent {
    pub fn new(media_type: impl Into<String>, data: impl Into<String>) -> Self {
        Self {
            encoding: "base64".into(),
            media_type: media_type.into(),
            data: data.into()
        }
    }

    pub fn media_type(&self) -> &str {
        &self.media_type
    }

    pub fn data(&self) -> Option<Vec<u8>> {
        BASE64_STANDARD.decode(&self.data).map_err(|err| {
            warn! { ?err };
            err
        }).ok()
    }
}

impl From<Image> for AnthropicImageContent {
    fn from(image: Image) -> Self {
        Self::new(&image.media_type, BASE64_STANDARD.encode(image.data))
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum AnthropicContent {
    #[serde(rename = "image")]
    Image { source: AnthropicImageContent },

    #[serde(rename = "text")]
    Text { text: String },
}

#[derive(Debug, Deserialize)]
pub struct AnthropicUsage {
    input_tokens: usize,
    output_tokens: usize,
}

impl AnthropicUsage {
    pub fn input_tokens(&self) -> usize {
        self.input_tokens
    }

    pub fn output_tokens(&self) -> usize {
        self.output_tokens
    }
}

#[derive(Debug, Deserialize)]
pub struct AnthropicMessageResponse {
    id: String,
    model: String,
    role: String,
    stop_reason: String,
    stop_sequence: Option<String>,
    usage: AnthropicUsage,
    content: Vec<AnthropicContent>
}

impl AnthropicMessageResponse {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    pub fn role(&self) -> &str {
        &self.role
    }

    pub fn stop_reason(&self) -> &str {
        &self.stop_reason
    }

    pub fn stop_sequence(&self) -> &Option<String> {
        &self.stop_sequence
    }

    pub fn usage(&self) -> &AnthropicUsage {
        &self.usage
    }

    pub fn content(&self) -> &Vec<AnthropicContent> {
        &self.content
    }
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum AnthropicResponse {
    #[serde(rename = "error")]
    Error { error: AnthropicErrorResponse },
    
    #[serde(rename = "message")]
    Message(AnthropicMessageResponse),
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum AnthropicMessageContent {
    Single(AnthropicContent),
    Multiple(Vec<AnthropicContent>),
}

#[derive(Debug, Serialize)]
struct AnthropicMessage {
    role: String,
    content: AnthropicMessageContent,
}

#[derive(Serialize)]
struct AnthropicRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    anthropic_version: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<String>,

    max_tokens: usize,
    messages: Vec<AnthropicMessage>,
    
    #[serde(skip_serializing_if = "Vec::is_empty")]
    stop_sequences: Vec<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,

    temperature: f32,
}

#[derive(Clone, Serialize)]
#[serde(untagged)]
pub enum AnthropicModel {
    Anthropic {
        api_key: String,
        api_version: String,
        model: String,
        max_tokens: usize,
        temperature: f32,
        
        #[serde(skip_serializing_if = "Vec::is_empty")]
        stop_sequences: Vec<String>,
        
        #[serde(skip_serializing_if = "Option::is_none")]
        system: Option<String>,

        #[serde(skip)]
        client: Client,
    },
    
    #[cfg(feature = "aws-bedrock")]
    Bedrock {
        #[serde(skip_serializing_if = "Option::is_none")]
        aws_config: Option<super::bedrock::AwsConfig>,

        api_version: String,
        model: String,
        max_tokens: usize,
        temperature: f32,
        
        #[serde(skip_serializing_if = "Vec::is_empty")]
        stop_sequences: Vec<String>,
        
        #[serde(skip_serializing_if = "Option::is_none")]
        system: Option<String>,

        #[serde(skip_serializing)]
        client: aws_sdk_bedrockruntime::Client,
    },
}

impl<'de> Deserialize<'de> for AnthropicModel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        const FIELDS: &[&str] = &["api_key", "api_version", "model", "max_tokens", "temperature", "stop_sequences", "system"];
        
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "snake_case")]
        enum Field { ApiKey, AwsConfig, ApiVersion, Model, MaxTokens, Temperature, StopSequences, System }

        struct AnthropicModelVisitor;

        impl<'de> Visitor<'de> for AnthropicModelVisitor {
            type Value = AnthropicModel;
            
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("enum AnthropicModel")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let mut api_version = None;
                let mut model = None;
                let mut max_tokens = None;
                let mut temperature = None;
                let mut stop_sequences = None;
                let mut system = None;

                let mut api_key = None;

                let mut aws_config = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::ApiKey => {
                            if aws_config.is_some() {
                                return Err(de::Error::custom("both `api_key` and `aws_config` should not be present"));
                            } else if api_key.is_some() {
                                return Err(de::Error::duplicate_field("api_key"));
                            }
                            api_key = Some(map.next_value()?);
                        },
                        Field::AwsConfig => {
                            if !cfg!(feature = "aws-bedrock") {
                                return Err(de::Error::unknown_field("aws_config", FIELDS));
                            } else if api_key.is_some() {
                                return Err(de::Error::custom("both `api_key` and `aws_config` should not be present"));
                            } else if aws_config.is_some() {
                                return Err(de::Error::duplicate_field("aws_config"));
                            }
                            aws_config = Some(map.next_value()?);
                        },
                        Field::ApiVersion => {
                            if api_version.is_some() {
                                return Err(de::Error::duplicate_field("api_version"));
                            }
                            api_version = Some(map.next_value()?);
                        }
                        Field::Model => {
                            if model.is_some() {
                                return Err(de::Error::duplicate_field("model"));
                            }
                            model = Some(map.next_value()?);
                        },
                        Field::MaxTokens => {
                            if max_tokens.is_some() {
                                return Err(de::Error::duplicate_field("max_tokens"));
                            }
                            max_tokens = Some(map.next_value()?);
                        },
                        Field::Temperature => {
                            if temperature.is_some() {
                                return Err(de::Error::duplicate_field("temperature"));
                            }
                            temperature = Some(map.next_value()?);
                        },
                        Field::StopSequences => {
                            if stop_sequences.is_some() {
                                return Err(de::Error::duplicate_field("stop_sequences"));
                            }
                            stop_sequences = Some(map.next_value()?);
                        },
                        Field::System => {
                            if system.is_some() {
                                return Err(de::Error::duplicate_field("system"));
                            }
                            system = Some(map.next_value()?);
                        }
                    }
                }

                if let Some(api_key) = api_key {
                    Ok(AnthropicModel::Anthropic {
                        api_key,
                        api_version: api_version.ok_or_else(|| de::Error::missing_field("api_version"))?,
                        model: model.ok_or_else(|| de::Error::missing_field("model"))?,
                        max_tokens: max_tokens.unwrap_or_else(|| 1024),
                        temperature: temperature.unwrap_or_else(|| 0.63),
                        stop_sequences: stop_sequences.unwrap_or_else(|| vec![]),
                        system,
                        client: Client::new(),
                    })
                } else {
                    #[cfg(feature = "aws-bedrock")]
                    {
                        let client = tokio::runtime::Runtime::new()
                            .map_err(|err| de::Error::custom(format!("{}", err)))?
                            .block_on(super::bedrock::bedrock_client(&aws_config));

                        Ok(AnthropicModel::Bedrock {
                            aws_config,
                            api_version: api_version.ok_or_else(|| de::Error::missing_field("api_version"))?,
                            model: model.ok_or_else(|| de::Error::missing_field("model"))?,
                            max_tokens: max_tokens.unwrap_or_else(|| 1024),
                            temperature: temperature.unwrap_or_else(|| 0.63),
                            stop_sequences: stop_sequences.unwrap_or_else(|| vec![]),
                            system,
                            client,
                        })
                    }

                    #[cfg(not(feature = "aws-bedrock"))]
                    Err(de::Error::missing_field("api_key"))
                }
            }
        }

        deserializer.deserialize_enum("AnthropicModel", &["Anthropic", "Bedrock"], AnthropicModelVisitor)
    }
}

impl AnthropicModel {
    pub fn new(api_key: impl Into<String>, api_version: impl Into<String>, model: impl Into<String>) -> Self {
        Self::Anthropic {
            api_key: api_key.into(),
            api_version: api_version.into(),
            model: model.into(),
            max_tokens: 1024,
            temperature: 0.63,
            stop_sequences: vec![],
            system: None,
            client: Client::new(),
        }
    }

    #[cfg(feature = "aws-bedrock")]
    pub async fn bedrock(api_version: impl Into<String>, model: impl Into<String>, aws_config: Option<super::bedrock::AwsConfig>) -> Self {
        let client = super::bedrock::bedrock_client(&aws_config).await;

        Self::Bedrock {
            aws_config,

            api_version: api_version.into(),
            model: model.into(),
            max_tokens: 1024,
            temperature: 0.63,
            stop_sequences: vec![],
            system: None,
            client,
        }
    }

    #[instrument(name = "AnthropicModel::max_tokens", level = "trace", skip(self))]
    pub fn max_tokens(self, max_tokens: usize) -> Self {
        match self {
            Self::Anthropic { api_key, api_version, model, max_tokens: _, temperature, stop_sequences, system, client } => Self::Anthropic { api_key, api_version, model, max_tokens, temperature, stop_sequences, system, client },
            
            #[cfg(feature = "aws-bedrock")]
            Self::Bedrock { aws_config, api_version, model, max_tokens: _, temperature, stop_sequences, system, client } => Self::Bedrock { aws_config, api_version, model, max_tokens, temperature, stop_sequences, system, client },
        }
    }

    #[instrument(name = "AnthropicModel::temperature", level = "trace", skip(self))]
    pub fn temperature(self, temperature: f32) -> Self {
        match self {
            Self::Anthropic { api_key, api_version, model, max_tokens, temperature: _, stop_sequences, system, client } => Self::Anthropic { api_key, api_version, model, max_tokens, temperature, stop_sequences, system, client },
            
            #[cfg(feature = "aws-bedrock")]
            Self::Bedrock { aws_config, api_version, model, max_tokens, temperature: _, stop_sequences, system, client } => Self::Bedrock { aws_config, api_version, model, max_tokens, temperature, stop_sequences, system, client },
        }
    }

    #[instrument(name = "AnthropicModel::stop_sequences", level = "trace", skip(self))]
    pub fn stop_sequences(self, stop_sequences: Vec<&str>) -> Self {
        match self {
            Self::Anthropic { api_key, api_version, model, max_tokens, temperature, stop_sequences: _, system, client } => Self::Anthropic { api_key, api_version, model, max_tokens, temperature, stop_sequences: stop_sequences.iter().map(|sequence| sequence.to_string()).collect::<Vec<String>>(), system, client },
            
            #[cfg(feature = "aws-bedrock")]
            Self::Bedrock { aws_config, api_version, model, max_tokens, temperature, stop_sequences: _, system, client } => Self::Bedrock { aws_config, api_version, model, max_tokens, temperature, stop_sequences: stop_sequences.iter().map(|sequence| sequence.to_string()).collect::<Vec<String>>(), system, client },
        }
    }

    #[instrument(name = "AnthropicModel::system", level = "trace", skip(self))]
    pub fn system(self, system: &str) -> Self {
        match self {
            Self::Anthropic { api_key, api_version, model, max_tokens, temperature, stop_sequences, system: _, client } => Self::Anthropic { api_key, api_version, model, max_tokens, temperature, stop_sequences, system: Some(system.to_string()), client },
            
            #[cfg(feature = "aws-bedrock")]
            Self::Bedrock { aws_config, api_version, model, max_tokens, temperature, stop_sequences, system: _, client } => Self::Bedrock { aws_config, api_version, model, max_tokens, temperature, stop_sequences, system: Some(system.to_string()), client },
        }
    }

    #[instrument(name = "AnthropicModel::create", level = "trace", skip(self))]
    pub async fn create(&self, messages: Vec<AnthropicContent>, conversation: Option<Vec<AnthropicMessage>>) -> Result<AnthropicMessageResponse, AnthropicErrorResponse> {
        let mut request_messages: Vec<AnthropicMessage> = vec![];
        if let Some(mut conversation) = conversation {
            request_messages.append(&mut conversation);
        }
        match messages.len() {
            0 => {},
            1 => request_messages.push(AnthropicMessage { role: "user".into(), content: AnthropicMessageContent::Single(messages[0].clone()) }),
            _ => request_messages.push(AnthropicMessage { role: "user".into(), content: AnthropicMessageContent::Multiple(messages.clone()) }),
        };

        match self {
            Self::Anthropic { api_key, api_version, model, max_tokens, temperature, stop_sequences, system, client } => {
                let request = AnthropicRequest {
                    anthropic_version: None,
                    model: Some(model.clone()),
                    max_tokens: max_tokens.clone(),
                    stop_sequences: stop_sequences.clone(),
                    system: system.clone(),
                    temperature: temperature.clone(),
    
                    messages: request_messages,
                };

                let response = client
                    .post("https://api.anthropic.com/v1/messages")
                    .header("x-api-key", api_key)
                    .header("anthropic-version", api_version)
                    .header("Accept", "application/json")
                    .header("Content-Type", "application/json")
                    .json(&request)
                    .send()
                    .await;

                match response {
                    Ok(response) => match response.status() {
                        StatusCode::OK => match response.json::<AnthropicResponse>().await {
                            Ok(response) => match response {
                                AnthropicResponse::Error { error } => Err(error),
                                AnthropicResponse::Message(message) => Ok(message)
                            },
                            Err(err) => Err(AnthropicErrorResponse { error_type: "invalid_response_error".into(), message: format!("{}", err) })
                        },
                        status_code if status_code.is_client_error() || status_code.is_server_error() => match response.json::<AnthropicResponse>().await {
                            Ok(response) => match response {
                                AnthropicResponse::Error { error } => Err(error),
                                AnthropicResponse::Message(message) => Err(AnthropicErrorResponse { error_type: "invalid_response_error".into(), message: format!("{:?}", message) })
                            },
                            Err(err) => Err(AnthropicErrorResponse { error_type: "invalid_response_error".into(), message: format!("{}", err) })
                        },
                        status_code => Err(AnthropicErrorResponse { error_type: "invalid_status_error".into(), message: format!("{}", status_code) })
                    },
                    Err(err) => Err(AnthropicErrorResponse { error_type: "request_error".into(), message: format!("{}", err) })
                }
            },

            #[cfg(feature = "aws-bedrock")]
            Self::Bedrock { aws_config: _, api_version, model, max_tokens, temperature, stop_sequences, system, client } => {
                let request = AnthropicRequest {
                    anthropic_version: Some(api_version.clone()),
                    model: None,
                    max_tokens: max_tokens.clone(),
                    stop_sequences: stop_sequences.clone(),
                    system: system.clone(),
                    temperature: temperature.clone(),
        
                    messages: request_messages,
                };

                let response = client.invoke_model()
                    .accept("application/json")
                    .content_type("application/json")
                    .model_id(model)
                    .body(aws_sdk_bedrockruntime::primitives::Blob::new(serde_json::to_vec(&request).map_err(|err| AnthropicErrorResponse { error_type: "request_error".into(), message: format!("{}", err) })?))
                    .send()
                    .await;

                match response {
                    Ok(response) => match serde_json::from_slice::<AnthropicResponse>(&response.body().clone().into_inner()) {
                        Ok(response) => match response {
                            AnthropicResponse::Error { error } => Err(error),
                            AnthropicResponse::Message(message) => Ok(message)
                        },
                        Err(err) => Err(AnthropicErrorResponse { error_type: "invalid_response_error".into(), message: format!("{}", err) })
                    },
                    Err(err) => Err(AnthropicErrorResponse { error_type: "bedrock_sdk_error".into(), message: format!("{}", err) })
                }
            },
        }
    }
}

impl LanguageModel for AnthropicModel {
    #[instrument(name = "AnthropicModel::inference", level = "trace", skip(self))]
    async fn inference(&self, prompt: &str, image: Option<Image>) -> Result<Message, Error> {
        let mut messages = vec![];
        if let Some(image) = image {
            messages.push(AnthropicContent::Image { source: image.into() });
        }
        messages.push(AnthropicContent::Text { text: prompt.into() });

        match self.create(messages, None).await.map(|message| {
            debug! { response = ?message };
            info! { usage = ?message.usage };

            message.content.first().and_then(|content| match content {
                AnthropicContent::Image { source } => match BASE64_STANDARD.decode(&source.data) {
                    Ok(data) => Ok(Message::Image(Image::new(&source.media_type, data))),
                    Err(err) => {
                        warn! { ?err };
                        Err(err)
                    }
                }.ok(),
                AnthropicContent::Text { text } => Some(Message::Text { text: text.clone() })
            })
        }) {
            Ok(message) => match message {
                Some(message) => Ok(message),
                None => Err(Error::Unexpected(anyhow!("no-content")))
            },
            Err(err) => {
                error! { ?err };
                Err(Error::ModelResponse(err.message))
            }
        }
    }
}