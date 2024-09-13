use aws_config::{profile::ProfileFileCredentialsProvider, Region};
use aws_credential_types::{
    provider::future,
    Credentials,
};
use aws_sdk_bedrockruntime::{
    config::{ProvideCredentials, SharedCredentialsProvider},
    Client,
};
use serde::{Deserialize, Serialize};

#[derive(Debug)]
struct CredentialParams {
    access_key: String,
    secret_key: String,
}

impl ProvideCredentials for CredentialParams {
    fn provide_credentials<'a>(&'a self) -> future::ProvideCredentials<'a>
    where
        Self: 'a
    {
        future::ProvideCredentials::ready(Ok(Credentials::new(self.access_key.clone(), self.secret_key.clone(), None, None, "ArgumentVariable")))
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum AwsConfig {
    Profile {
        profile_name: String,

        #[serde(skip_serializing_if = "Option::is_none")]
        region: Option<String>,
    },
    Credential {
        #[serde(skip_serializing_if = "Option::is_none")]
        access_key: Option<String>,
        
        #[serde(skip_serializing_if = "Option::is_none")]
        secret_key: Option<String>,
        
        #[serde(skip_serializing_if = "Option::is_none")]
        region: Option<String>,
    }
}

pub async fn bedrock_client(aws_config: &Option<AwsConfig>) -> Client {
    let sdk_config = if let Some(aws_config) = aws_config {
        match aws_config {
            AwsConfig::Credential { access_key, secret_key, region } => {
                if (access_key.is_some() && secret_key.is_some()) || region.is_some() {
                    let mut builder = aws_config::load_from_env().await.into_builder();

                    if let (Some(access_key), Some(secret_key)) = (access_key, secret_key) {
                        builder = builder.credentials_provider(SharedCredentialsProvider::new(CredentialParams {
                            access_key: access_key.clone(),
                            secret_key: secret_key.clone(),
                        }));
                    }

                    if let Some(region) = region {
                        builder = builder.region(Region::new(region.clone()));
                    }

                    builder.build()
                } else {
                    aws_config::load_from_env().await
                }
            },
            AwsConfig::Profile { profile_name, region } => {
                let mut builder = aws_config::load_from_env().await.into_builder();

                builder = builder.credentials_provider(SharedCredentialsProvider::new(ProfileFileCredentialsProvider::builder().profile_name(profile_name).build()));

                if let Some(region) = region {
                    builder = builder.region(Region::new(region.clone()));
                }

                builder.build()
            },
        }
    } else {
        aws_config::load_from_env().await
    };

    Client::new(&sdk_config)
}