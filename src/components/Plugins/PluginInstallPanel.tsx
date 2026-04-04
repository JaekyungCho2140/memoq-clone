import { useRef, useState } from "react";
import { fileRefFromDrop } from "../../adapters";
import type { PluginInstallRequest } from "../../types";

interface PluginInstallPanelProps {
  installing: boolean;
  onInstall(req: PluginInstallRequest): void;
}

export function PluginInstallPanel({ installing, onInstall }: PluginInstallPanelProps) {
  const fileInputRef = useRef<HTMLInputElement>(null);
  const [selectedFile, setSelectedFile] = useState<File | null>(null);
  const [dragOver, setDragOver] = useState(false);

  const handleFileChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0] ?? null;
    setSelectedFile(file);
  };

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault();
    setDragOver(false);
    const file = e.dataTransfer.files[0];
    if (file && file.name.endsWith(".wasm")) {
      setSelectedFile(file);
    }
  };

  const handleInstall = () => {
    if (!selectedFile) return;
    const wasmFile = fileRefFromDrop(selectedFile);
    onInstall({ wasmFile });
    setSelectedFile(null);
    if (fileInputRef.current) fileInputRef.current.value = "";
  };

  return (
    <div className="plugin-install-panel">
      <div
        className={`plugin-drop-zone${dragOver ? " plugin-drop-zone--active" : ""}`}
        onDragOver={(e) => { e.preventDefault(); setDragOver(true); }}
        onDragLeave={() => setDragOver(false)}
        onDrop={handleDrop}
        onClick={() => fileInputRef.current?.click()}
        role="button"
        tabIndex={0}
        onKeyDown={(e) => e.key === "Enter" && fileInputRef.current?.click()}
      >
        <input
          ref={fileInputRef}
          type="file"
          accept=".wasm"
          style={{ display: "none" }}
          onChange={handleFileChange}
        />
        {selectedFile ? (
          <span className="plugin-drop-filename">{selectedFile.name}</span>
        ) : (
          <span className="plugin-drop-hint">
            .wasm 파일을 여기에 드래그하거나 클릭하여 선택
          </span>
        )}
      </div>

      <button
        className="btn-small"
        onClick={handleInstall}
        disabled={!selectedFile || installing}
      >
        {installing ? "설치 중..." : "설치"}
      </button>
    </div>
  );
}
