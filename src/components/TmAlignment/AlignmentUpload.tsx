import { useState, useRef } from "react";
import { registerFile } from "../../adapters/web";
import { useAlignmentStore } from "../../stores/alignmentStore";
import { useTmStore } from "../../stores/tmStore";
import { adapter } from "../../adapters";

const SUPPORTED_EXTENSIONS = [".docx", ".txt", ".xliff"];

const LANG_OPTIONS = [
  { value: "en", label: "English" },
  { value: "ko", label: "Korean" },
  { value: "ja", label: "Japanese" },
  { value: "zh", label: "Chinese" },
  { value: "de", label: "German" },
  { value: "fr", label: "French" },
  { value: "es", label: "Spanish" },
];

function isValidFile(file: File): boolean {
  return SUPPORTED_EXTENSIONS.some((ext) => file.name.toLowerCase().endsWith(ext));
}

function FileDropZone({
  label,
  file,
  onFile,
}: {
  label: string;
  file: File | null;
  onFile: (f: File) => void;
}) {
  const inputRef = useRef<HTMLInputElement>(null);
  const [dragging, setDragging] = useState(false);

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault();
    setDragging(false);
    const dropped = e.dataTransfer.files[0];
    if (dropped && isValidFile(dropped)) onFile(dropped);
  };

  return (
    <div
      className={`alignment-dropzone${dragging ? " dragging" : ""}${file ? " has-file" : ""}`}
      onDragOver={(e) => { e.preventDefault(); setDragging(true); }}
      onDragLeave={() => setDragging(false)}
      onDrop={handleDrop}
      onClick={() => inputRef.current?.click()}
    >
      <input
        ref={inputRef}
        type="file"
        accept={SUPPORTED_EXTENSIONS.join(",")}
        style={{ display: "none" }}
        onChange={(e) => {
          const f = e.target.files?.[0];
          if (f && isValidFile(f)) onFile(f);
        }}
      />
      <div className="dropzone-label">{label}</div>
      {file ? (
        <div className="dropzone-filename">{file.name}</div>
      ) : (
        <div className="dropzone-hint">클릭하거나 드래그하여 파일을 선택하세요<br />{SUPPORTED_EXTENSIONS.join(", ")}</div>
      )}
    </div>
  );
}

export function AlignmentUpload() {
  const { sourceLang, targetLang, tmId, setConfig, setPhase, setPairs, setProgress, setError } =
    useAlignmentStore();
  const { activeTmId, setActiveTmId } = useTmStore();

  const [sourceFile, setSourceFile] = useState<File | null>(null);
  const [targetFile, setTargetFile] = useState<File | null>(null);
  const [localSourceLang, setLocalSourceLang] = useState(sourceLang);
  const [localTargetLang, setLocalTargetLang] = useState(targetLang);
  const [tmName, setTmName] = useState("Aligned TM");
  const [loading, setLoading] = useState(false);
  const [localError, setLocalError] = useState<string | null>(null);

  const canStart = sourceFile !== null && targetFile !== null && localSourceLang !== localTargetLang;

  const handleStart = async () => {
    if (!canStart || loading) return;
    setLoading(true);
    setLocalError(null);

    try {
      // Ensure a TM exists
      let currentTmId = activeTmId ?? tmId;
      if (!currentTmId) {
        currentTmId = await adapter.createTm(tmName, localSourceLang, localTargetLang);
        setActiveTmId(currentTmId);
      }

      setConfig(localSourceLang, localTargetLang, currentTmId);
      setPhase("processing");
      setProgress(0);

      // Simulate progress while waiting for alignment
      const progressInterval = setInterval(() => {
        setProgress((p) => Math.min(p + 10, 85));
      }, 300);

      const sourceRef = registerFile(sourceFile!);
      const targetRef = registerFile(targetFile!);

      const result = await adapter.alignmentAlign({
        sourceFileRef: sourceRef,
        targetFileRef: targetRef,
        sourceLang: localSourceLang,
        targetLang: localTargetLang,
        tmId: currentTmId,
      });

      clearInterval(progressInterval);
      setProgress(100);
      setPairs(result.pairs);
      setPhase("review");
    } catch (err) {
      setPhase("upload");
      const msg = err instanceof Error ? err.message : "정렬 처리 중 오류가 발생했습니다.";
      setLocalError(msg);
      setError(msg);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="alignment-upload">
      <h2 className="alignment-section-title">TM 정렬 — 파일 업로드</h2>
      <p className="alignment-description">
        소스 파일과 타겟 파일을 선택하면 자동으로 문장을 정렬하여 TM 항목을 생성합니다.
      </p>

      <div className="alignment-lang-row">
        <div className="alignment-lang-field">
          <label>소스 언어</label>
          <select
            value={localSourceLang}
            onChange={(e) => setLocalSourceLang(e.target.value)}
          >
            {LANG_OPTIONS.map((o) => (
              <option key={o.value} value={o.value}>{o.label}</option>
            ))}
          </select>
        </div>
        <div className="alignment-arrow">→</div>
        <div className="alignment-lang-field">
          <label>타겟 언어</label>
          <select
            value={localTargetLang}
            onChange={(e) => setLocalTargetLang(e.target.value)}
          >
            {LANG_OPTIONS.map((o) => (
              <option key={o.value} value={o.value}>{o.label}</option>
            ))}
          </select>
        </div>
      </div>

      {localSourceLang === localTargetLang && (
        <p className="alignment-warning">소스 언어와 타겟 언어가 다르게 설정되어야 합니다.</p>
      )}

      <div className="alignment-dropzone-row">
        <FileDropZone label="소스 파일" file={sourceFile} onFile={setSourceFile} />
        <FileDropZone label="타겟 파일" file={targetFile} onFile={setTargetFile} />
      </div>

      <div className="alignment-tm-field">
        <label>TM 이름 (신규 생성 시)</label>
        <input
          type="text"
          value={tmName}
          onChange={(e) => setTmName(e.target.value)}
          placeholder="Aligned TM"
        />
      </div>

      {localError && <p className="alignment-error">{localError}</p>}

      <div className="alignment-actions">
        <button
          className="btn btn-primary"
          onClick={handleStart}
          disabled={!canStart || loading}
        >
          {loading ? "처리 중..." : "정렬 시작"}
        </button>
      </div>
    </div>
  );
}
