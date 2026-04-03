import { useState, FormEvent } from "react";
import { useAuthStore } from "../../stores/authStore";

export function LoginPage() {
  const { login, isLoading, error, setError } = useAuthStore();
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault();
    if (!username.trim() || !password.trim()) {
      setError("사용자 이름과 비밀번호를 입력하세요.");
      return;
    }
    try {
      await login(username.trim(), password);
    } catch {
      // error already set in store
    }
  };

  return (
    <div className="login-page">
      <div className="login-card">
        <div className="login-header">
          <span className="home-logo-icon">⚡</span>
          <h1 className="login-title">memoQ Clone</h1>
          <p className="login-subtitle">웹 에디터 로그인</p>
        </div>

        <form className="login-form" onSubmit={handleSubmit} noValidate>
          <div className="login-field">
            <label className="login-label" htmlFor="login-username">사용자 이름</label>
            <input
              id="login-username"
              className="login-input"
              type="text"
              autoComplete="username"
              autoFocus
              disabled={isLoading}
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              placeholder="username"
            />
          </div>

          <div className="login-field">
            <label className="login-label" htmlFor="login-password">비밀번호</label>
            <input
              id="login-password"
              className="login-input"
              type="password"
              autoComplete="current-password"
              disabled={isLoading}
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder="••••••••"
            />
          </div>

          {error && <p className="login-error" role="alert">{error}</p>}

          <button
            className="login-btn"
            type="submit"
            disabled={isLoading}
          >
            {isLoading ? "로그인 중..." : "로그인"}
          </button>
        </form>
      </div>
    </div>
  );
}
