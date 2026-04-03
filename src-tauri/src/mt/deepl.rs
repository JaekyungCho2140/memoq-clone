use crate::mt::{engine::MtProvider, MtError, MtProviderInfo};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

pub struct DeepLProvider;

#[derive(Serialize)]
struct DeepLRequest<'a> {
    text: Vec<&'a str>,
    source_lang: &'a str,
    target_lang: &'a str,
}

#[derive(Deserialize)]
struct DeepLResponse {
    translations: Vec<DeepLTranslation>,
}

#[derive(Deserialize)]
struct DeepLTranslation {
    text: String,
}

#[derive(Deserialize)]
struct DeepLErrorBody {
    message: Option<String>,
}

const DEEPL_FREE_URL: &str = "https://api-free.deepl.com/v2/translate";
const DEEPL_PRO_URL: &str = "https://api.deepl.com/v2/translate";

impl DeepLProvider {
    fn api_url(api_key: &str) -> &'static str {
        // Free keys end with ":fx"
        if api_key.ends_with(":fx") {
            DEEPL_FREE_URL
        } else {
            DEEPL_PRO_URL
        }
    }
}

#[async_trait]
impl MtProvider for DeepLProvider {
    fn info(&self) -> MtProviderInfo {
        MtProviderInfo {
            id: "deepl".to_string(),
            name: "DeepL".to_string(),
            requires_api_key: true,
        }
    }

    async fn translate(
        &self,
        text: &str,
        source_lang: &str,
        target_lang: &str,
        api_key: &str,
    ) -> Result<String, MtError> {
        let url = Self::api_url(api_key);
        let client = Client::new();

        let resp = client
            .post(url)
            .header("Authorization", format!("DeepL-Auth-Key {}", api_key))
            .json(&DeepLRequest {
                text: vec![text],
                source_lang: &source_lang.to_uppercase(),
                target_lang: &target_lang.to_uppercase(),
            })
            .send()
            .await
            .map_err(MtError::Http)?;

        let status = resp.status();

        if status.as_u16() == 429 {
            return Err(MtError::RateLimit);
        }

        if !status.is_success() {
            let err: DeepLErrorBody = resp
                .json()
                .await
                .unwrap_or(DeepLErrorBody { message: None });
            return Err(MtError::Api {
                code: status.as_u16(),
                message: err
                    .message
                    .unwrap_or_else(|| format!("HTTP {}", status.as_u16())),
            });
        }

        let body: DeepLResponse = resp.json().await.map_err(MtError::Http)?;
        body.translations
            .into_iter()
            .next()
            .map(|t| t.text)
            .ok_or(MtError::Api {
                code: 200,
                message: "Empty translation response".to_string(),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;

    #[tokio::test]
    async fn test_deepl_translate_success() {
        let mut server = Server::new_async().await;
        let _m = server
            .mock("POST", "/v2/translate")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"translations":[{"text":"Hallo Welt","detected_source_language":"EN"}]}"#,
            )
            .create_async()
            .await;

        let client = Client::new();
        let url = format!("{}/v2/translate", server.url());
        let resp = client
            .post(&url)
            .header("Authorization", "DeepL-Auth-Key test:fx")
            .json(&DeepLRequest {
                text: vec!["Hello World"],
                source_lang: "EN",
                target_lang: "DE",
            })
            .send()
            .await
            .unwrap();

        assert!(resp.status().is_success());
        let body: DeepLResponse = resp.json().await.unwrap();
        assert_eq!(body.translations[0].text, "Hallo Welt");
    }

    #[tokio::test]
    async fn test_deepl_translate_rate_limit() {
        let mut server = Server::new_async().await;
        let _m = server
            .mock("POST", "/v2/translate")
            .with_status(429)
            .with_body(r#"{"message":"Too many requests"}"#)
            .create_async()
            .await;

        let client = Client::new();
        let url = format!("{}/v2/translate", server.url());
        let resp = client
            .post(&url)
            .header("Authorization", "DeepL-Auth-Key test:fx")
            .json(&DeepLRequest {
                text: vec!["Hello"],
                source_lang: "EN",
                target_lang: "DE",
            })
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status().as_u16(), 429);
    }

    #[tokio::test]
    async fn test_deepl_api_url_free_key() {
        assert_eq!(DeepLProvider::api_url("abc123:fx"), DEEPL_FREE_URL);
        assert_eq!(DeepLProvider::api_url("abc123"), DEEPL_PRO_URL);
    }
}
