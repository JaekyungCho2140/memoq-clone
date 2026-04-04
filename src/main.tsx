import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./styles/global.css";
import { useProjectStore } from "./stores/projectStore";
import { useAuthStore } from "./stores/authStore";
import type { Project } from "./types";
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
  };
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <ErrorBoundary>
      <App />
    </ErrorBoundary>
  </React.StrictMode>
);
