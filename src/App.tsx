import { useProjectStore } from "./stores/projectStore";
import { ProjectView } from "./components/ProjectView/ProjectView";
import { EditorLayout } from "./components/Layout/EditorLayout";

export default function App() {
  const project = useProjectStore((s) => s.project);
  return (
    <div className="app-root">
      {project ? <EditorLayout /> : <ProjectView />}
    </div>
  );
}
