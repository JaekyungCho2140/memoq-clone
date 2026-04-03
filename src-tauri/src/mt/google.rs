use crate::mt::{engine::MtProvider, MtError, MtProviderInfo};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

pub struct GoogleProvider;

#[derive(Serialize)]
struct GoogleRequest<'a> {
    q: &'a str,
    source: &'a str,
    target: &'a str,
    format: &'a str,
    key: &'a str,
}

#[derive(Deserialize)]
struct GoogleResponse {
    data: GoogleData,
}

#[derive(Deserialize)]
struct GoogleData {
    translations: Vec<GoogleTranslation>,
}

#[derive(Deserialize)]
struct GoogleTranslation {
    #[serde(rename = "translatedText")]
    translated_text: String,
}

#[derive(Deserialize)]
struct GoogleErrorResponse {
    error: Option<GoogleErrorDetail>,
}

#[derive(Deserialize)]
struct GoogleErrorDetail {
    code: u16,
    message: String,
}

const GOOGLE_TRANSLATE_URL: &str = "https://translation.googleapis.com/language/translate/v2";

#[async_trait]
impl MtProvider for GoogleProvider {
    fn info(&self) -> MtProviderInfo {
        MtProviderInfo {
            id: "google".to_string(),
            name: "Google Translate".to_string(),
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
        let client = Client::new();

        let resp = client
            .post(GOOGLE_TRANSLATE_URL)
            .json(&GoogleRequest {
                q: text,
                source: source_lang,
                target: target_lang,
                format: "text",
                key: api_key,
            })
            .send()
            .await
            .map_err(MtError::Http)?;

        let status = resp.status();

        if status.as_u16() == 429 {
            return Err(MtError::RateLimit);
        }

        if !status.is_success() {
            let err: GoogleErrorResponse = resp
                .json()
                .await
                .unwrap_or(GoogleErrorResponse { error: None });
            let (code, msg) = err
                .error
                .map(|e| (e.code, e.message))
                .unwrap_or((status.as_u16(), format!("HTTP {}", status.as_u16())));
            return Err(MtError::Api { code, message: msg });
        }

        let body: GoogleResponse = resp.json().await.map_err(MtError::Http)?;
        body.data
            .translations
            .into_iter()
            .next()
            .map(|t| t.translated_text)
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
    async fn test_google_translate_success() {
        let mut server = Server::new_async().await;
        let _m = server
            .mock("POST", "/language/translate/v2")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"data":{"translations":[{"translatedText":"Hallo Welt"}]}}"#)
            .create_async()
            .await;

        let client = Client::new();
        let url = format!("{}/language/translate/v2", server.url());
        let resp = client
            .post(&url)
            .json(&GoogleRequest {
                q: "Hello World",
                source: "en",
                target: "de",
                format: "text",
                key: "test-key",
            })
            .send()
            .await
            .unwrap();

        assert!(resp.status().is_success());
        let body: GoogleResponse = resp.json().await.unwrap();
        assert_eq!(body.data.translations[0].translated_text, "Hallo Welt");
    }

    #[tokio::test]
    async fn test_google_translate_error() {
        let mut server = Server::new_async().await;
        let _m = server
            .mock("POST", "/language/translate/v2")
            .with_status(400)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"error":{"code":400,"message":"Invalid API key","status":"INVALID_ARGUMENT"}}"#,
            )
            .create_async()
            .await;

        let client = Client::new();
        let url = format!("{}/language/translate/v2", server.url());
        let resp = client
            .post(&url)
            .json(&GoogleRequest {
                q: "Hello",
                source: "en",
                target: "de",
                format: "text",
                key: "bad-key",
            })
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status().as_u16(), 400);
        let err: GoogleErrorResponse = resp.json().await.unwrap();
        assert_eq!(err.error.unwrap().message, "Invalid API key");
    }
}
