/**
 * Adapter entry point.
 *
 * Detects the runtime environment at startup and exports the appropriate
 * adapter singleton.  All application code should import from here:
 *
 *   import { adapter } from "../../adapters";
 *
 * Environment detection:
 *   - `window.__TAURI__` is injected by the Tauri runtime.
 *   - Its absence means we are running in a plain browser.
 */

import { tauriAdapter } from "./tauri";
import { webAdapter, registerFile as webRegisterFile } from "./web";
import type { IAppAdapter, FileRef } from "./types";

export type { IAppAdapter, OpenFileDialogOptions, SaveFileDialogOptions, FileRef } from "./types";

export function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI__" in window;
}

export const adapter: IAppAdapter = isTauri() ? tauriAdapter : webAdapter;

/**
 * Create a FileRef from a browser File object dropped onto a drag target.
 *
 * In Tauri mode the file object has a `.path` property injected by the Tauri
 * drag-and-drop handler; use that directly.
 * In web mode the File is registered in the in-memory file registry and a
 * synthetic `web-file://` URI is returned.
 */
export function fileRefFromDrop(file: File): FileRef {
  if (isTauri()) {
    const path = (file as unknown as { path?: string }).path;
    return path ?? file.name;
  }
  return webRegisterFile(file);
}
