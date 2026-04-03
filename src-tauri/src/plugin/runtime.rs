//! wasmtime-based WASM plugin sandbox runtime.
//!
//! `PluginRuntime` loads a compiled WASM binary (targeting `wasm32-wasip1`),
//! instantiates it inside a WASI-preview-1 sandbox, and provides typed call
//! helpers for each plugin kind.

use std::path::Path;

use anyhow::{Context, Result};
use plugin_api::{
    MtRequest, MtResponse, ParseRequest, ParseResponse, PluginMetadata, PluginResult, QaRequest,
    QaResponse,
};
use wasmtime::{Engine, Linker, Module, Store};
use wasmtime_wasi::preview1::{self, WasiP1Ctx};
use wasmtime_wasi::WasiCtxBuilder;

// ──────────────────────────────────────────────
// Store state
// ──────────────────────────────────────────────

struct PluginState {
    wasi: WasiP1Ctx,
}

// ──────────────────────────────────────────────
// PluginRuntime
// ──────────────────────────────────────────────

/// A loaded, sandboxed WASM plugin instance.
pub struct PluginRuntime {
    engine: Engine,
    module: Module,
}

impl PluginRuntime {
    /// Load a WASM binary from disk.
    pub fn from_file(path: &Path) -> Result<Self> {
        let engine = Engine::default();
        let module = Module::from_file(&engine, path)
            .with_context(|| format!("loading WASM module {:?}", path))?;
        Ok(Self { engine, module })
    }

    /// Load a WASM binary from bytes (useful for testing).
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let engine = Engine::default();
        let module =
            Module::new(&engine, bytes).with_context(|| "compiling WASM module from bytes")?;
        Ok(Self { engine, module })
    }

    // ── internal helpers ────────────────────────────────────────────────

    fn make_store(&self) -> Result<Store<PluginState>> {
        let wasi = WasiCtxBuilder::new().inherit_stdio().build_p1();
        let state = PluginState { wasi };
        Ok(Store::new(&self.engine, state))
    }

    fn make_linker(&self) -> Result<Linker<PluginState>> {
        let mut linker: Linker<PluginState> = Linker::new(&self.engine);
        preview1::add_to_linker_sync(&mut linker, |s| &mut s.wasi)?;
        Ok(linker)
    }

    /// Call an exported plugin function that takes JSON input and returns JSON
    /// output via the `(ptr << 32 | len)` ABI.
    fn call_json<I: serde::Serialize, O: serde::de::DeserializeOwned>(
        &self,
        export_name: &str,
        input: &I,
    ) -> Result<O> {
        let mut store = self.make_store()?;
        let linker = self.make_linker()?;
        let instance = linker
            .instantiate(&mut store, &self.module)
            .with_context(|| "instantiating WASM module")?;

        // ── allocate guest memory for input ────────────────────────────
        let alloc = instance
            .get_typed_func::<(i32,), (i32,)>(&mut store, "alloc")
            .with_context(|| "plugin is missing 'alloc' export")?;
        let dealloc = instance
            .get_typed_func::<(i32, i32), ()>(&mut store, "dealloc")
            .with_context(|| "plugin is missing 'dealloc' export")?;

        let input_json = serde_json::to_vec(input)?;
        let input_len = input_json.len() as i32;
        let (input_ptr,) = alloc.call(&mut store, (input_len,))?;

        // write bytes into linear memory
        {
            let memory = instance
                .get_memory(&mut store, "memory")
                .with_context(|| "plugin is missing 'memory' export")?;
            memory.write(&mut store, input_ptr as usize, &input_json)?;
        }

        // ── call the export ────────────────────────────────────────────
        let func = instance
            .get_typed_func::<(i32, i32), (i64,)>(&mut store, export_name)
            .with_context(|| format!("plugin is missing '{}' export", export_name))?;
        let (ret,) = func.call(&mut store, (input_ptr, input_len))?;

        // free input buffer
        dealloc.call(&mut store, (input_ptr, input_len))?;

        // ── decode output ──────────────────────────────────────────────
        let out_ptr = ((ret as u64) >> 32) as usize;
        let out_len = (ret & 0xffff_ffff) as usize;

        let output_json: Vec<u8> = {
            let memory = instance
                .get_memory(&mut store, "memory")
                .with_context(|| "plugin is missing 'memory' export")?;
            let mut buf = vec![0u8; out_len];
            memory.read(&store, out_ptr, &mut buf)?;
            buf
        };

        // free output buffer
        dealloc.call(&mut store, (out_ptr as i32, out_len as i32))?;

        let result: PluginResult<O> = serde_json::from_slice(&output_json)?;
        match result {
            PluginResult::Ok(v) => Ok(v),
            PluginResult::Err { error } => anyhow::bail!("plugin error: {}", error),
        }
    }

    /// Call a zero-argument export that returns a packed (ptr << 32 | len) JSON pointer.
    fn call_zero_arg_json<O: serde::de::DeserializeOwned>(&self, export_name: &str) -> Result<O> {
        let mut store = self.make_store()?;
        let linker = self.make_linker()?;
        let instance = linker
            .instantiate(&mut store, &self.module)
            .with_context(|| "instantiating WASM module")?;

        let dealloc = instance
            .get_typed_func::<(i32, i32), ()>(&mut store, "dealloc")
            .with_context(|| "plugin is missing 'dealloc' export")?;
        let func = instance
            .get_typed_func::<(), (i64,)>(&mut store, export_name)
            .with_context(|| format!("plugin is missing '{}' export", export_name))?;
        let (ret,) = func.call(&mut store, ())?;

        let out_ptr = ((ret as u64) >> 32) as usize;
        let out_len = (ret & 0xffff_ffff) as usize;

        let json: Vec<u8> = {
            let memory = instance
                .get_memory(&mut store, "memory")
                .with_context(|| "plugin is missing 'memory' export")?;
            let mut buf = vec![0u8; out_len];
            memory.read(&store, out_ptr, &mut buf)?;
            buf
        };
        dealloc.call(&mut store, (out_ptr as i32, out_len as i32))?;

        Ok(serde_json::from_slice(&json)?)
    }

    // ── public API ───────────────────────────────────────────────────────

    /// Read plugin metadata from the `plugin_metadata` export.
    pub fn metadata(&self) -> Result<PluginMetadata> {
        self.call_zero_arg_json("plugin_metadata")
    }

    /// Call the MT provider's `mt_translate` export.
    pub fn mt_translate(&self, req: MtRequest) -> Result<MtResponse> {
        self.call_json("mt_translate", &req)
    }

    /// Call the file parser's `parse_file` export.
    pub fn parse_file(&self, req: ParseRequest) -> Result<ParseResponse> {
        self.call_json("parse_file", &req)
    }

    /// Call the QA rule's `qa_check` export.
    pub fn qa_check(&self, req: QaRequest) -> Result<QaResponse> {
        self.call_json("qa_check", &req)
    }
}

// ──────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal valid WASM module — used to verify that a module that lacks
    /// required exports returns a descriptive error rather than panicking.
    const EMPTY_WASM: &[u8] = &[
        0x00, 0x61, 0x73, 0x6D, // magic
        0x01, 0x00, 0x00, 0x00, // version
    ];

    #[test]
    fn from_bytes_accepts_valid_wasm() {
        let rt = PluginRuntime::from_bytes(EMPTY_WASM);
        assert!(rt.is_ok(), "expected Ok, got Err");
    }

    #[test]
    fn from_bytes_rejects_garbage() {
        let rt = PluginRuntime::from_bytes(b"not wasm");
        assert!(rt.is_err());
    }

    #[test]
    fn mt_translate_errors_on_missing_export() {
        let rt = PluginRuntime::from_bytes(EMPTY_WASM).unwrap();
        let req = MtRequest {
            source_lang: "en".into(),
            target_lang: "ko".into(),
            segments: vec!["Hello".into()],
        };
        let result = rt.mt_translate(req);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("alloc") || msg.contains("export") || msg.contains("dealloc"),
            "unexpected error: {}",
            msg
        );
    }

    #[test]
    fn qa_check_errors_on_missing_export() {
        let rt = PluginRuntime::from_bytes(EMPTY_WASM).unwrap();
        let req = QaRequest {
            segments: vec![plugin_api::QaSegment {
                id: 1,
                source: "Hello".into(),
                target: "안녕".into(),
            }],
        };
        assert!(rt.qa_check(req).is_err());
    }

    #[test]
    fn parse_file_errors_on_missing_export() {
        let rt = PluginRuntime::from_bytes(EMPTY_WASM).unwrap();
        let req = ParseRequest {
            file_bytes_b64: "AAAA".into(),
            mime_hint: "text/plain".into(),
        };
        assert!(rt.parse_file(req).is_err());
    }

    #[test]
    fn metadata_errors_on_missing_export() {
        let rt = PluginRuntime::from_bytes(EMPTY_WASM).unwrap();
        assert!(rt.metadata().is_err());
    }
}
