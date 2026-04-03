//! Host-side plugin trait definitions.
//!
//! These are the Rust interfaces that `PluginRuntime` implements by
//! dispatching to the loaded WASM module.

use anyhow::Result;
use plugin_api::{MtRequest, MtResponse, ParseRequest, ParseResponse, QaRequest, QaResponse};

/// A machine-translation provider plugin.
pub trait MtProviderPlugin: Send + Sync {
    fn translate(&self, req: MtRequest) -> Result<MtResponse>;
}

/// A file-format parser plugin.
pub trait FileParserPlugin: Send + Sync {
    fn parse(&self, req: ParseRequest) -> Result<ParseResponse>;
}

/// A QA rule plugin.
pub trait QaRulePlugin: Send + Sync {
    fn check(&self, req: QaRequest) -> Result<QaResponse>;
}
