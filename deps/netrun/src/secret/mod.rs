use anyhow::Result;
use infisical::{AuthMethod, Client, secrets::GetSecretRequest};

pub const US_INFISICAL_URL: &str = "https://app.infisical.com";
pub const EU_INFISICAL_URL: &str = "https://eu.infisical.com";

pub struct SecretsManager {
    client:      Client,
    project_id:  String,
    environment: String,
}

impl SecretsManager {
    pub async fn new(
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
        project_id: impl Into<String>,
        environment: impl Into<String>,
    ) -> Result<Self> {
        let mut client = Client::builder().base_url(EU_INFISICAL_URL).build().await?;

        let auth_method = AuthMethod::new_universal_auth(client_id, client_secret);

        client.login(auth_method).await?;

        Ok(Self {
            client,
            project_id: project_id.into(),
            environment: environment.into(),
        })
    }

    pub async fn get(&self, key: impl Into<String>) -> Result<String> {
        let request = GetSecretRequest::builder(key, &self.project_id, &self.environment).build();

        let secret = self.client.secrets().get(request).await?;

        Ok(secret.secret_value)
    }
}

#[cfg(test)]
mod test {
    use std::env::var;

    use anyhow::{Context, Result};
    use tokio::sync::OnceCell;

    use crate::secret::SecretsManager;

    async fn secrets() -> Result<&'static SecretsManager> {
        static SECRETS: OnceCell<SecretsManager> = OnceCell::const_new();

        SECRETS
            .get_or_try_init(|| async {
                let client_secret = var("INFISICAL_NETRUN").context("INFISICAL_NETRUN")?;

                let manager = SecretsManager::new(
                    "b51b4908-b9f9-4a43-ac20-796908c9f80f",
                    client_secret,
                    "ba83e490-eb77-4376-a23a-9348a53cd381",
                    "dev",
                )
                .await
                .context("Secrets Manager init")?;

                Ok(manager)
            })
            .await
    }

    #[ignore = "needs the Infisical client secret in .env"]
    #[tokio::test]
    async fn test_secret() -> Result<()> {
        dotenvy::dotenv()?;

        let manager = SecretsManager::new(
            "b51b4908-b9f9-4a43-ac20-796908c9f80f",
            var("INFISICAL_NETRUN")?,
            "ba83e490-eb77-4376-a23a-9348a53cd381",
            "dev",
        )
        .await?;

        assert_eq!(manager.get("TEST_SECRET").await?, "plati");

        Ok(())
    }

    #[ignore = "needs the Infisical client secret in .env"]
    #[tokio::test]
    async fn test_global_managersecret() -> Result<()> {
        dotenvy::dotenv()?;

        assert_eq!(secrets().await?.get("TEST_SECRET").await?, "plati");

        Ok(())
    }
}
