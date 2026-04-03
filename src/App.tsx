import { useEffect } from "react";
import { isTauri } from "./adapters";
import { useAuthStore } from "./stores/authStore";
import { useProjectStore } from "./stores/projectStore";
import { LoginPage } from "./components/Auth/LoginPage";
import { HomePage } from "./components/Home/HomePage";
import { ProjectDashboard } from "./components/ProjectDashboard/ProjectDashboard";
import { EditorLayout } from "./components/Layout/EditorLayout";

export default function App() {
  const project = useProjectStore((s) => s.project);
  const projectView = useProjectStore((s) => s.projectView);
  const { user, accessToken, rehydrate } = useAuthStore();

  // In web mode, restore auth session from localStorage on first render
  useEffect(() => {
    if (!isTauri()) {
      rehydrate();
    }
  }, []);

  // Web mode: require login
  if (!isTauri() && !user && !accessToken) {
    return <LoginPage />;
  }

  if (!project) return <HomePage />;
  if (projectView === "dashboard") return <ProjectDashboard />;
  return <EditorLayout />;
}
