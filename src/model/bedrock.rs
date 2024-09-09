use aws_config::Region;
use aws_credential_types::{
    provider::future,
    Credentials,
};
use aws_sdk_bedrockruntime::{
    config::{ProvideCredentials, SharedCredentialsProvider},
    Client,
};

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

pub async fn bedrock_client(aws_access_key: Option<String>, aws_secret_key: Option<String>, aws_region: Option<String>) -> Client {
    let mut sdk_config = aws_config::load_from_env().await;

    if let (Some(access_key), Some(secret_key)) = (&aws_access_key, &aws_secret_key) {
        sdk_config = sdk_config.into_builder()
            .credentials_provider(SharedCredentialsProvider::new(CredentialParams {
                access_key: access_key.clone(),
                secret_key: secret_key.clone(),
            }))
            .build();
    }

    if let Some(region) = &aws_region {
        sdk_config = sdk_config.into_builder()
            .region(Region::new(region.clone()))
            .build();
    }

    Client::new(&sdk_config)
}