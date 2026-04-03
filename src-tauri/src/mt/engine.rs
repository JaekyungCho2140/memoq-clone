use crate::mt::{MtError, MtProviderInfo};
use async_trait::async_trait;
use keyring::Entry;

const KEYCHAIN_SERVICE: &str = "memoq-clone-mt";

#[async_trait]
pub trait MtProvider: Send + Sync {
    fn info(&self) -> MtProviderInfo;
    async fn translate(
        &self,
        text: &str,
        source_lang: &str,
        target_lang: &str,
        api_key: &str,
    ) -> Result<String, MtError>;
}

pub fn get_providers() -> Vec<MtProviderInfo> {
    vec![
        crate::mt::deepl::DeepLProvider.info(),
        crate::mt::google::GoogleProvider.info(),
    ]
}

pub async fn translate(
    text: &str,
    source_lang: &str,
    target_lang: &str,
    provider_id: &str,
) -> Result<String, MtError> {
    let api_key = load_api_key(provider_id)?;
    match provider_id {
        "deepl" => {
            crate::mt::deepl::DeepLProvider
                .translate(text, source_lang, target_lang, &api_key)
                .await
        }
        "google" => {
            crate::mt::google::GoogleProvider
                .translate(text, source_lang, target_lang, &api_key)
                .await
        }
        other => Err(MtError::Api {
            code: 400,
            message: format!("Unknown provider: {}", other),
        }),
    }
}

pub fn save_api_key(provider: &str, api_key: &str) -> Result<(), MtError> {
    let entry =
        Entry::new(KEYCHAIN_SERVICE, provider).map_err(|e| MtError::Keychain(e.to_string()))?;
    entry
        .set_password(api_key)
        .map_err(|e| MtError::Keychain(e.to_string()))
}

pub fn load_api_key(provider: &str) -> Result<String, MtError> {
    let entry =
        Entry::new(KEYCHAIN_SERVICE, provider).map_err(|e| MtError::Keychain(e.to_string()))?;
    entry.get_password().map_err(|_| MtError::InvalidApiKey)
}
