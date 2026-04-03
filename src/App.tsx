import { useProjectStore } from "./stores/projectStore";
import { HomePage } from "./components/Home/HomePage";
import { ProjectDashboard } from "./components/ProjectDashboard/ProjectDashboard";
import { EditorLayout } from "./components/Layout/EditorLayout";

export default function App() {
  const project = useProjectStore((s) => s.project);
  const projectView = useProjectStore((s) => s.projectView);

  if (!project) return <HomePage />;
  if (projectView === "dashboard") return <ProjectDashboard />;
  return <EditorLayout />;
}
