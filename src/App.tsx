import { useEffect, useState } from "react";
import { isTauri } from "./adapters";
import { useAuthStore } from "./stores/authStore";
import { useProjectStore } from "./stores/projectStore";
import { useVendorStore } from "./stores/vendorStore";
import { LoginPage } from "./components/Auth/LoginPage";
import { HomePage } from "./components/Home/HomePage";
import { ProjectDashboard } from "./components/ProjectDashboard/ProjectDashboard";
import { EditorLayout } from "./components/Layout/EditorLayout";
import { VendorDashboard } from "./components/Vendor/VendorDashboard";
import { AdminVendorDashboard } from "./components/Vendor/AdminVendorDashboard";
import { VendorEditorLayout } from "./components/Vendor/VendorEditorLayout";
import { TmAlignmentPage } from "./components/TmAlignment/TmAlignmentPage";
import { UpdateChecker } from "./components/Updater/UpdateChecker";

export default function App() {
  const project = useProjectStore((s) => s.project);
  const projectView = useProjectStore((s) => s.projectView);
  const { user, accessToken, rehydrate } = useAuthStore();
  const { activeAssignment } = useVendorStore();
  const [showAlignment, setShowAlignment] = useState(false);
  const [showVendorMgmt, setShowVendorMgmt] = useState(false);

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

  // TM Alignment page (modal-like full-page view)
  if (showAlignment) {
    return <TmAlignmentPage onClose={() => setShowAlignment(false)} />;
  }

  // Admin vendor management page
  if (!isTauri() && user?.role === "admin" && showVendorMgmt) {
    return <AdminVendorDashboard onClose={() => setShowVendorMgmt(false)} />;
  }

  // Vendor role: dedicated portal UI
  if (!isTauri() && user?.role === "vendor") {
    if (activeAssignment) return <VendorEditorLayout />;
    return <VendorDashboard />;
  }

  // Admin / Tauri: full app
  const appContent = !project ? (
    <HomePage
      onOpenAlignment={() => setShowAlignment(true)}
      onOpenVendorManagement={!isTauri() && user?.role === "admin" ? () => setShowVendorMgmt(true) : undefined}
    />
  ) : projectView === "dashboard" ? (
    <ProjectDashboard />
  ) : (
    <EditorLayout />
  );
  return (
    <>
      <UpdateChecker />
      {appContent}
    </>
  );
}
