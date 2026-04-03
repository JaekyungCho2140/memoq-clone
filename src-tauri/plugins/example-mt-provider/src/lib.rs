//! Example custom MT provider plugin.
//!
//! This plugin simulates a simple "echo" MT provider that returns each source
//! segment prefixed with `[MT] `.  It demonstrates how to implement the
//! memoq-clone plugin ABI.
//!
//! # Build
//!
//! ```sh
//! cargo build --target wasm32-wasip1 --release -p example-mt-provider
//! # Output: target/wasm32-wasip1/release/example_mt_provider.wasm
//! ```
//!
//! Copy the resulting `.wasm` to a plugin directory alongside `manifest.json`.

use plugin_api::{
    MtRequest, MtResponse,
    PluginKind, PluginMetadata, PluginResult,
};

// ── Memory helpers (required by the ABI) ────────────────────────────────────

/// Allocate `size` bytes on the WASM heap and return the pointer.
#[no_mangle]
pub extern "C" fn alloc(size: i32) -> i32 {
    let mut buf = Vec::<u8>::with_capacity(size as usize);
    let ptr = buf.as_mut_ptr() as i32;
    std::mem::forget(buf);
    ptr
}

/// Free a previously-allocated buffer.
#[no_mangle]
pub extern "C" fn dealloc(ptr: i32, len: i32) {
    unsafe {
        let _ = Vec::from_raw_parts(ptr as *mut u8, len as usize, len as usize);
    }
}

// ── Helper: write JSON to heap and return packed (ptr << 32 | len) ──────────

fn write_json<T: serde::Serialize>(value: &T) -> i64 {
    let json = serde_json::to_vec(value).unwrap_or_default();
    let len = json.len();
    let mut buf = json;
    buf.shrink_to_fit();
    let ptr = buf.as_mut_ptr() as i64;
    std::mem::forget(buf);
    (ptr << 32) | (len as i64)
}

// ── Helper: read JSON from guest memory ─────────────────────────────────────

unsafe fn read_json<T: serde::de::DeserializeOwned>(ptr: i32, len: i32) -> Option<T> {
    let slice = std::slice::from_raw_parts(ptr as *const u8, len as usize);
    serde_json::from_slice(slice).ok()
}

// ── Plugin exports ───────────────────────────────────────────────────────────

/// Return plugin metadata.
#[no_mangle]
pub extern "C" fn plugin_metadata() -> i64 {
    let meta = PluginMetadata {
        id: "com.example.echo-mt".into(),
        name: "Echo MT Provider".into(),
        version: "0.1.0".into(),
        author: "memoq-clone team".into(),
        description: "Example plugin: echoes each segment with a [MT] prefix.".into(),
        kind: PluginKind::MtProvider,
    };
    write_json(&meta)
}

/// Translate segments (echo strategy: prepend `[MT] `).
///
/// # Safety
/// `ptr`/`len` must point to valid UTF-8 JSON within the module's linear memory.
#[no_mangle]
pub unsafe extern "C" fn mt_translate(ptr: i32, len: i32) -> i64 {
    let req: MtRequest = match read_json(ptr, len) {
        Some(r) => r,
        None => {
            let err: PluginResult<MtResponse> = PluginResult::Err {
                error: "invalid JSON input".into(),
            };
            return write_json(&err);
        }
    };

    let translations = req
        .segments
        .iter()
        .map(|s| format!("[MT] {}", s))
        .collect();

    let resp: PluginResult<MtResponse> = PluginResult::Ok(MtResponse { translations });
    write_json(&resp)
}
