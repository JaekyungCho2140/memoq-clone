import { useEffect, useState } from "react";
import { useMtStore } from "../../stores/mtStore";
import { adapter } from "../../adapters";
import type { MtProvider } from "../../types";

export function MtSettings() {
  const { provider, apiKey, setProvider, setApiKey, clearCache } = useMtStore();
  const [saving, setSaving] = useState(false);
  const [testing, setTesting] = useState(false);
  const [testResult, setTestResult] = useState<string | null>(null);
  const [loaded, setLoaded] = useState(false);

  // Load saved settings on mount
  useEffect(() => {
    adapter.mtLoadSettings()
      .then((saved) => {
        if (saved) {
          setProvider(saved.provider);
          setApiKey(saved.apiKey);
        }
      })
      .catch(() => {/* backend not yet available */})
      .finally(() => setLoaded(true));
  }, []);

  const handleSave = async () => {
    setSaving(true);
    setTestResult(null);
    try {
      await adapter.mtSaveSettings({ provider, apiKey });
      clearCache();
      setTestResult("✓ 저장되었습니다.");
    } catch {
      setTestResult("✗ 저장 실패 — 백엔드가 아직 준비 중입니다 (AFR-16 대기).");
    } finally {
      setSaving(false);
    }
  };

  const handleTest = async () => {
    if (!apiKey.trim()) {
      setTestResult("API 키를 입력해 주세요.");
      return;
    }
    setTesting(true);
    setTestResult(null);
    try {
      const result = await adapter.mtTranslate({
        source: "Hello",
        sourceLang: "en",
        targetLang: "ko",
        provider,
        apiKey,
      });
      setTestResult(`✓ 연결 성공 — 결과: "${result.target}"`);
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      if (msg.includes("mt_translate")) {
        setTestResult("✗ 백엔드가 아직 준비 중입니다 (AFR-16 대기).");
      } else {
        setTestResult(`✗ 연결 실패 — ${msg}`);
      }
    } finally {
      setTesting(false);
    }
  };

  if (!loaded) return null;

  return (
    <div className="mt-settings">
      <h4 className="mt-settings-title">MT (기계 번역) 설정</h4>

      <div className="mt-settings-row">
        <label className="mt-settings-label">프로바이더</label>
        <select
          className="mt-settings-select"
          value={provider}
          onChange={(e) => setProvider(e.target.value as MtProvider)}
        >
          <option value="deepl">DeepL</option>
          <option value="google">Google 번역</option>
        </select>
      </div>

      <div className="mt-settings-row">
        <label className="mt-settings-label">API 키</label>
        <input
          type="password"
          className="mt-settings-input"
          value={apiKey}
          onChange={(e) => setApiKey(e.target.value)}
          placeholder={provider === "deepl" ? "DeepL API 키 입력..." : "Google API 키 입력..."}
          autoComplete="off"
        />
      </div>

      <div className="mt-settings-actions">
        <button
          className="btn-small"
          onClick={handleSave}
          disabled={saving || testing}
        >
          {saving ? "저장 중..." : "저장"}
        </button>
        <button
          className="btn-small btn-outline"
          onClick={handleTest}
          disabled={saving || testing || !apiKey.trim()}
        >
          {testing ? "테스트 중..." : "연결 테스트"}
        </button>
      </div>

      {testResult && (
        <div className={`mt-test-result ${testResult.startsWith("✓") ? "mt-test-ok" : "mt-test-err"}`}>
          {testResult}
        </div>
      )}
    </div>
  );
}
