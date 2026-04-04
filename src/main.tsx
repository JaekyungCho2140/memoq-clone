import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./styles/global.css";
import { useProjectStore } from "./stores/projectStore";
import { useAuthStore } from "./stores/authStore";
import { useTmStore } from "./stores/tmStore";
import type { Project, TmMatch } from "./types";
import { ErrorBoundary } from "./components/ErrorBoundary";

// Expose smoke test utilities in non-production builds.
// Tests can call window.__smokeTest.setProject(project) to load a fixture
// without going through the file-open dialog.
if (import.meta.env.DEV) {
  (window as Window & {
    __smokeTest?: {
      setProject: (p: Project) => void;
      openEditor: () => void;
      setAdminAuth: () => void;
      setTmMatches: (matches: TmMatch[]) => void;
    }
  }).__smokeTest = {
    setProject: (p: Project) => useProjectStore.getState().setProject(p),
    openEditor: () => useProjectStore.getState().openEditor(),
    /** Bypass web-mode login gate by injecting a synthetic admin session. */
    setAdminAuth: () =>
      useAuthStore.setState({
        user: { id: "smoke-test-user", username: "smoke", role: "admin" },
        accessToken: "smoke-test-token",
      }),
    /** Directly inject TM matches into the store (bypasses adapter.searchTm). */
    setTmMatches: (matches: TmMatch[]) =>
      useTmStore.getState().setCurrentTmMatches(matches),
  };
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <ErrorBoundary>
      <App />
    </ErrorBoundary>
  </React.StrictMode>
);
