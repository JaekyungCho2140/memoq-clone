import { useEffect, useState } from "react";
import { useProjectStore } from "../../stores/projectStore";
import { useTmStore } from "../../stores/tmStore";
import { useMtStore } from "../../stores/mtStore";
import { adapter } from "../../adapters";
import type { TmMatch, MtResult, LiveDocsMatch } from "../../types";

type ActiveTab = "tm" | "mt" | "livedocs";

export function TmPanel() {
  const { project, currentSegmentIndex, updateSegment } = useProjectStore();
  const { activeTmId, setActiveTmId, setCurrentTmMatches } = useTmStore();
  const { provider, apiKey, cacheResult, getCached } = useMtStore();
  const [matches, setMatches] = useState<TmMatch[]>([]);
  const [creating, setCreating] = useState(false);
  const [tmName, setTmName] = useState("");
  const [adding, setAdding] = useState(false);
  const [activeTab, setActiveTab] = useState<ActiveTab>("tm");
  const [mtResult, setMtResult] = useState<MtResult | null>(null);
  const [mtLoading, setMtLoading] = useState(false);
  const [liveDocsMatches, setLiveDocsMatches] = useState<LiveDocsMatch[]>([]);
  const [liveDocsLoading, setLiveDocsLoading] = useState(false);

  const currentSegment = project?.segments[currentSegmentIndex];

  // Auto-create a TM for the project's language pair if none exists
  useEffect(() => {
    if (!activeTmId && project) {
      const defaultName = `${project.sourceLang} → ${project.targetLang}`;
      adapter.createTm(defaultName, project.sourceLang, project.targetLang)
        .then((id) => setActiveTmId(id))
        .catch(() => {/* ignore */});
    }
  }, [project?.id]);

  // Search TM when current segment changes
  useEffect(() => {
    if (!activeTmId || !currentSegment || !project) {
      setMatches([]);
      return;
    }
    adapter.searchTm({
      tmId: activeTmId,
      query: currentSegment.source,
      sourceLang: project.sourceLang,
      targetLang: project.targetLang,
      minScore: 0.5,
    })
      .then((results) => {
        setMatches(results);
        setCurrentTmMatches(results);
      })
      .catch(() => {
        setMatches([]);
        setCurrentTmMatches([]);
      });
  }, [activeTmId, currentSegment?.id, project?.id]);

  // Load cached MT result when segment changes
  useEffect(() => {
    if (!currentSegment) {
      setMtResult(null);
      return;
    }
    const cached = getCached(currentSegment.id);
    setMtResult(cached);
  }, [currentSegment?.id]);

  // Auto-search LiveDocs when on livedocs tab and segment changes
  useEffect(() => {
    if (activeTab !== "livedocs" || !currentSegment?.source.trim()) {
      setLiveDocsMatches([]);
      return;
    }
    setLiveDocsLoading(true);
    adapter.liveDocsListLibraries()
      .then(async (libs) => {
        if (libs.length === 0) return [];
        const results = await Promise.all(
          libs.map((lib) =>
            adapter.liveDocsSearch(currentSegment.source, lib.id, 0.5)
              .catch(() => [] as LiveDocsMatch[])
          )
        );
        return results.flat().sort((a, b) => b.score - a.score);
      })
      .then(setLiveDocsMatches)
      .catch(() => setLiveDocsMatches([]))
      .finally(() => setLiveDocsLoading(false));
  }, [activeTab, currentSegment?.id]);

  const applyMatch = (target: string) => {
    if (!currentSegment) return;
    updateSegment(currentSegment.id, { target, status: "draft" });
  };

  const handleCreateTm = async () => {
    if (!tmName.trim() || !project) return;
    try {
      const id = await adapter.createTm(tmName.trim(), project.sourceLang, project.targetLang);
      setActiveTmId(id);
      setTmName("");
      setCreating(false);
    } catch (e) {
      console.error("Failed to create TM:", e);
    }
  };

  const handleAddToTm = async () => {
    if (!activeTmId || !currentSegment || !project) return;
    if (!currentSegment.target.trim()) return;
    setAdding(true);
    try {
      await adapter.addToTm(
        activeTmId,
        currentSegment.source,
        currentSegment.target,
        project.sourceLang,
        project.targetLang,
      );
      // Refresh matches
      const fresh = await adapter.searchTm({
        tmId: activeTmId,
        query: currentSegment.source,
        sourceLang: project.sourceLang,
        targetLang: project.targetLang,
        minScore: 0.5,
      });
      setMatches(fresh);
    } catch (e) {
      console.error("Failed to add to TM:", e);
    } finally {
      setAdding(false);
    }
  };

  const handleMtFetch = async () => {
    if (!currentSegment || !project || mtLoading) return;
    if (!apiKey.trim()) return;
    setMtLoading(true);
    try {
      const result = await adapter.mtTranslate({
        source: currentSegment.source,
        sourceLang: project.sourceLang,
        targetLang: project.targetLang,
        provider,
        apiKey,
      });
      cacheResult(currentSegment.id, result);
      setMtResult(result);
    } catch {
      /* backend not yet ready */
    } finally {
      setMtLoading(false);
    }
  };

  return (
    <div className="tm-panel">
      <div className="panel-header">
        <div className="panel-tabs">
          <button
            className={`panel-tab${activeTab === "tm" ? " active" : ""}`}
            onClick={() => setActiveTab("tm")}
          >
            TM
          </button>
          <button
            className={`panel-tab${activeTab === "mt" ? " active" : ""}`}
            onClick={() => setActiveTab("mt")}
          >
            MT
          </button>
          <button
            className={`panel-tab${activeTab === "livedocs" ? " active" : ""}`}
            onClick={() => setActiveTab("livedocs")}
          >
            LiveDocs
          </button>
        </div>
        <div className="panel-actions">
          {activeTab === "tm" && currentSegment?.target && (
            <button
              className="btn-small"
              onClick={handleAddToTm}
              disabled={adding}
              title="현재 세그먼트를 TM에 추가"
            >
              {adding ? "..." : "+ TM"}
            </button>
          )}
          {activeTab === "tm" && (
            <button
              className="btn-small btn-outline"
              onClick={() => setCreating((v) => !v)}
              title="새 TM 만들기"
            >
              새 TM
            </button>
          )}
        </div>
      </div>

      {activeTab === "tm" && (
        <>
          {creating && (
            <div className="create-form">
              <input
                type="text"
                value={tmName}
                onChange={(e) => setTmName(e.target.value)}
                placeholder="TM 이름"
                onKeyDown={(e) => e.key === "Enter" && handleCreateTm()}
                autoFocus
              />
              <button className="btn-small" onClick={handleCreateTm}>만들기</button>
              <button className="btn-small btn-outline" onClick={() => setCreating(false)}>취소</button>
            </div>
          )}

          {matches.length === 0 ? (
            <p className="no-matches">매치 없음</p>
          ) : (
            matches.map((m, i) => (
              <div key={i} className="tm-match" onClick={() => applyMatch(m.target)}>
                <div className="match-score">{Math.round(m.score * 100)}%</div>
                <div className="match-source">{m.source}</div>
                <div className="match-target">{m.target}</div>
              </div>
            ))
          )}
        </>
      )}

      {activeTab === "livedocs" && (
        <div className="livedocs-tab">
          {!currentSegment ? (
            <p className="no-matches">세그먼트를 선택하세요.</p>
          ) : liveDocsLoading ? (
            <p className="mt-tab-loading">검색 중...</p>
          ) : liveDocsMatches.length === 0 ? (
            <p className="no-matches">LiveDocs 매치 없음</p>
          ) : (
            liveDocsMatches.map((m, i) => (
              <div key={i} className="tm-match" onClick={() => applyMatch(m.sentence)}>
                <div className="match-score">{Math.round(m.score * 100)}%</div>
                <div className="match-source">{m.sentence}</div>
                <div className="match-target livedocs-doc-path">{m.docPath}</div>
              </div>
            ))
          )}
        </div>
      )}

      {activeTab === "mt" && (
        <div className="mt-tab">
          {!apiKey.trim() ? (
            <p className="no-matches">MT API 키가 설정되지 않았습니다.</p>
          ) : mtLoading ? (
            <p className="mt-tab-loading">번역 중...</p>
          ) : mtResult ? (
            <div
              className="tm-match mt-match"
              onClick={() => applyMatch(mtResult.target)}
              title="클릭하여 적용"
            >
              <div className="match-score mt-label">{mtResult.provider.toUpperCase()}</div>
              <div className="match-source">{mtResult.source}</div>
              <div className="match-target">{mtResult.target}</div>
            </div>
          ) : (
            <div className="mt-tab-empty">
              <p className="no-matches">MT 결과 없음</p>
              <button className="btn-small" onClick={handleMtFetch}>
                MT 번역 가져오기
              </button>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
