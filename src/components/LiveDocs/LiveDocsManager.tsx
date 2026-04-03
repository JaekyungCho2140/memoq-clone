import { useEffect, useState } from "react";
import { adapter } from "../../adapters";
import type { LiveDocsLibrary } from "../../types";

export function LiveDocsManager() {
  const [libraries, setLibraries] = useState<LiveDocsLibrary[]>([]);
  const [newLibName, setNewLibName] = useState("");
  const [creating, setCreating] = useState(false);
  const [indexingLibId, setIndexingLibId] = useState<string | null>(null);
  const [indexingStatus, setIndexingStatus] = useState<Record<string, string>>({});

  useEffect(() => {
    adapter.liveDocsListLibraries()
      .then(setLibraries)
      .catch(() => {/* backend may not be ready */});
  }, []);

  const handleCreateLibrary = async () => {
    if (!newLibName.trim()) return;
    try {
      const lib = await adapter.liveDocsCreateLibrary(newLibName.trim());
      setLibraries((prev) => [...prev, lib]);
      setNewLibName("");
      setCreating(false);
    } catch (e) {
      console.error("라이브러리 생성 실패:", e);
    }
  };

  const handleAddDocument = async (libId: string) => {
    try {
      const fileRef = await adapter.openFileDialog({
        multiple: false,
        filters: [{ name: "Text/Document", extensions: ["txt", "docx", "tmx", "xliff"] }],
      });
      if (!fileRef) return;

      setIndexingLibId(libId);
      setIndexingStatus((prev) => ({ ...prev, [libId]: "indexing" }));

      const updated = await adapter.liveDocsAddDocument(libId, fileRef);
      setLibraries((prev) => prev.map((l) => (l.id === libId ? updated : l)));
      setIndexingStatus((prev) => ({ ...prev, [libId]: "done" }));
    } catch (e) {
      console.error("문서 추가 실패:", e);
      setIndexingStatus((prev) => ({ ...prev, [libId]: "error" }));
    } finally {
      setIndexingLibId(null);
    }
  };

  return (
    <div className="livedocs-manager">
      <div className="livedocs-manager-header">
        <h3>LiveDocs 라이브러리</h3>
        <button
          className="btn-small btn-outline"
          onClick={() => setCreating((v) => !v)}
        >
          + 새 라이브러리
        </button>
      </div>

      {creating && (
        <div className="create-form">
          <input
            type="text"
            value={newLibName}
            onChange={(e) => setNewLibName(e.target.value)}
            placeholder="라이브러리 이름"
            onKeyDown={(e) => e.key === "Enter" && handleCreateLibrary()}
            autoFocus
          />
          <button className="btn-small" onClick={handleCreateLibrary}>만들기</button>
          <button
            className="btn-small btn-outline"
            onClick={() => setCreating(false)}
          >
            취소
          </button>
        </div>
      )}

      {libraries.length === 0 ? (
        <p className="no-matches">라이브러리가 없습니다.</p>
      ) : (
        libraries.map((lib) => (
          <div key={lib.id} className="livedocs-library">
            <div className="livedocs-library-header">
              <span className="livedocs-library-name">{lib.name}</span>
              <button
                className="btn-small"
                onClick={() => handleAddDocument(lib.id)}
                disabled={indexingLibId === lib.id}
              >
                {indexingLibId === lib.id ? "인덱싱 중..." : "+ 문서 추가"}
              </button>
            </div>

            {indexingLibId === lib.id && (
              <div className="indexing-status">
                <span className="indexing-spinner" /> 인덱싱 중...
              </div>
            )}
            {indexingStatus[lib.id] === "done" && indexingLibId !== lib.id && (
              <div className="indexing-done">✓ 인덱싱 완료</div>
            )}
            {indexingStatus[lib.id] === "error" && (
              <div className="indexing-error">문서 추가에 실패했습니다.</div>
            )}

            {lib.documents.length === 0 ? (
              <p className="no-matches">문서 없음</p>
            ) : (
              <ul className="livedocs-doc-list">
                {lib.documents.map((doc) => (
                  <li key={doc.id} className="livedocs-doc-item">
                    <span className="livedocs-doc-path">{doc.path}</span>
                    <span className="livedocs-doc-count">
                      {doc.sentences.length}문장
                    </span>
                  </li>
                ))}
              </ul>
            )}
          </div>
        ))
      )}
    </div>
  );
}
