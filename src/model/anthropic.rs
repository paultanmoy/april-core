use anyhow::{anyhow, Result};
use base64::prelude::{BASE64_STANDARD, Engine as _};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, instrument, warn};

use super::{Image, LanguageModel, Message};

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
    #[serde(rename = "anthropic-version", skip_serializing_if = "Option::is_none")]
    api_version: Option<String>,

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

#[derive(Deserialize, Serialize)]
pub enum AnthropicModel {
    Anthropic {
        api_key: String,
        api_version: String,
        model: String,
        max_tokens: usize,
        temperature: f32,
        stop_sequences: Vec<String>,
        system: Option<String>,
    },
    Bedrock {
        aws_access_key: String,
        aws_secret_key: String,
        aws_region: String,
        api_version: String,
        model: String,
        max_tokens: usize,
        temperature: f32,
        stop_sequences: Vec<String>,
        system: Option<String>,
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
        }
    }

    pub fn bedrock(aws_access_key: impl Into<String>, aws_secret_key: impl Into<String>, aws_region: impl Into<String>, api_version: impl Into<String>, model: impl Into<String>) -> Self {
        Self::Bedrock {
            aws_access_key: aws_access_key.into(),
            aws_secret_key: aws_secret_key.into(),
            aws_region: aws_region.into(),
            api_version: api_version.into(),
            model: model.into(),
            max_tokens: 1024,
            temperature: 0.63,
            stop_sequences: vec![],
            system: None,
        }
    }

    #[instrument(name = "AnthropicModel:max_tokens", level = "trace", skip(self))]
    pub fn max_tokens(self, max_tokens: usize) -> Self {
        match self {
            Self::Anthropic { api_key, api_version, model, max_tokens: _, temperature, stop_sequences, system } => Self::Anthropic { api_key, api_version, model, max_tokens: max_tokens, temperature, stop_sequences, system },
            Self::Bedrock { aws_access_key, aws_secret_key, aws_region, api_version, model, max_tokens: _, temperature, stop_sequences, system } => Self::Bedrock { aws_access_key, aws_secret_key, aws_region, api_version, model, max_tokens: max_tokens, temperature, stop_sequences, system },
        }
    }

    #[instrument(name = "AnthropicModel:temperature", level = "trace", skip(self))]
    pub fn temperature(self, temperature: f32) -> Self {
        match self {
            Self::Anthropic { api_key, api_version, model, max_tokens, temperature: _, stop_sequences, system } => Self::Anthropic { api_key, api_version, model, max_tokens, temperature: temperature, stop_sequences, system },
            Self::Bedrock { aws_access_key, aws_secret_key, aws_region, api_version, model, max_tokens, temperature: _, stop_sequences, system } => Self::Bedrock { aws_access_key, aws_secret_key, aws_region, api_version, model, max_tokens, temperature: temperature, stop_sequences, system },
        }
    }

    #[instrument(name = "AnthropicModel:stop_sequences", level = "trace", skip(self))]
    pub fn stop_sequences(self, stop_sequences: Vec<&str>) -> Self {
        let sequences = stop_sequences.iter().map(|sequence| sequence.to_string()).collect::<Vec<String>>();
        match self {
            Self::Anthropic { api_key, api_version, model, max_tokens, temperature, stop_sequences: _, system } => Self::Anthropic { api_key, api_version, model, max_tokens, temperature, stop_sequences: sequences, system },
            Self::Bedrock { aws_access_key, aws_secret_key, aws_region, api_version, model, max_tokens, temperature, stop_sequences: _, system } => Self::Bedrock { aws_access_key, aws_secret_key, aws_region, api_version, model, max_tokens, temperature, stop_sequences: sequences, system },
        }
    }

    #[instrument(name = "AnthropicModel:system", level = "trace", skip(self))]
    pub fn system(self, system: &str) -> Self {
        match self {
            Self::Anthropic { api_key, api_version, model, max_tokens, temperature, stop_sequences, system: _ } => Self::Anthropic { api_key, api_version, model, max_tokens, temperature, stop_sequences, system: Some(system.into()) },
            Self::Bedrock { aws_access_key, aws_secret_key, aws_region, api_version, model, max_tokens, temperature, stop_sequences, system: _ } => Self::Bedrock { aws_access_key, aws_secret_key, aws_region, api_version, model, max_tokens, temperature, stop_sequences, system: Some(system.into()) },
        }
    }

    #[instrument(name = "AnthropicModel:create", level = "trace", skip(self))]
    pub async fn create(&self, messages: Vec<AnthropicContent>, conversation: Option<Vec<AnthropicMessage>>) -> Result<AnthropicMessageResponse, AnthropicErrorResponse> {
        let client = Client::new();

        let mut request_messages: Vec<AnthropicMessage> = vec![];
        if let Some(mut conversation) = conversation {
            request_messages.append(&mut conversation);
        }
        match messages.len() {
            0 => {},
            1 => request_messages.push(AnthropicMessage { role: "user".into(), content: AnthropicMessageContent::Single(messages[0].clone()) }),
            _ => request_messages.push(AnthropicMessage { role: "user".into(), content: AnthropicMessageContent::Multiple(messages.clone()) }),
        };

        let response = match self {
            Self::Anthropic { api_key, api_version, model, max_tokens, temperature, stop_sequences, system } => {
                let request = AnthropicRequest {
                    api_version: None,
                    model: Some(model.clone()),
                    max_tokens: max_tokens.clone(),
                    stop_sequences: stop_sequences.clone(),
                    system: system.clone(),
                    temperature: temperature.clone(),

                    messages: request_messages,
                };
                
                client.post("https://api.anthropic.com/v1/messages")
                    .header("x-api-key", api_key)
                    .header("anthropic-version", api_version)
                    .header("Content-Type", "application/json")
                    .json(&request)
                    .send()
                    .await
            },
            Self::Bedrock { aws_access_key, aws_secret_key, aws_region, api_version, model, max_tokens, temperature, stop_sequences, system } => {
                let request = AnthropicRequest {
                    api_version: Some(api_version.clone()),
                    model: None,
                    max_tokens: max_tokens.clone(),
                    stop_sequences: stop_sequences.clone(),
                    system: system.clone(),
                    temperature: temperature.clone(),

                    messages: request_messages,
                };

                client.post("https://api.anthropic.com/v1/messages")
                    .header("Content-Type", "application/json")
                    .json(&request)
                    .send()
                    .await
            },
        };
        match response {
            Ok(response) => match response.status() {
                StatusCode::OK => match response.json::<AnthropicResponse>().await {
                    Ok(response) => match response {
                        AnthropicResponse::Error { error } => Err(error),
                        AnthropicResponse::Message(message) => Ok(message)
                    },
                    Err(err) => Err(AnthropicErrorResponse { error_type: "invalid_response_error".into(), message: format!("{}", err) })
                },
                status_code if status_code.is_client_error() => match response.json::<AnthropicResponse>().await {
                    Ok(response) => match response {
                        AnthropicResponse::Error { error } => Err(error),
                        AnthropicResponse::Message(message) => Err(AnthropicErrorResponse { error_type: "invalid_response_error".into(), message: format!("{:?}", message) })
                    },
                    Err(err) => Err(AnthropicErrorResponse { error_type: "invalid_response_error".into(), message: format!("{}", err) })
                },
                status_code => Err(AnthropicErrorResponse { error_type: "invalid_status_error".into(), message: format!("{}", status_code) })
            },
            Err(err) => {
                error! { ?err };
                panic!()
            }
        }
    }
}

impl LanguageModel for AnthropicModel {
    #[instrument(name = "AnthropicModel:inference", level = "trace", skip(self))]
    async fn inference(&self, prompt: &str, image: Option<Image>) -> Result<Message> {
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
                None => Err(anyhow!("no-content"))
            },
            Err(err) => {
                error! { ?err };
                Err(anyhow!(err.message))
            }
        }
    }
}