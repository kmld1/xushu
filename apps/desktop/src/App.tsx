import { HashRouter, Navigate, Route, Routes } from "react-router";

import { AppLayout } from "@/app/app-layout";
import { AppProviders } from "@/app/providers";
import { AppErrorBoundary } from "@/components/app-error-boundary";
import { DataProtectionPage } from "@/pages/data-protection-page";
import { NotFoundPage } from "@/pages/not-found-page";
import { OverviewPage } from "@/pages/overview-page";
import { DiagnosticsPage } from "@/pages/persistence-page";
import { ProjectDetailPage } from "@/pages/project-detail-page";
import { ProjectKnowledgePage } from "@/pages/project-knowledge-page";
import { ProjectsPage } from "@/pages/projects-page";
import { RecoveryPage } from "@/pages/recovery-page";
import { SettingsPage } from "@/pages/settings-page";
import { TaskDetailPage } from "@/pages/task-detail-page";
import { TasksPage } from "@/pages/tasks-page";
import { VerificationPage } from "@/pages/verification-page";

function App() {
  return (
    <AppErrorBoundary>
      <AppProviders>
        <HashRouter>
          <Routes>
            <Route element={<AppLayout />}>
              <Route index element={<OverviewPage />} />
              <Route path="projects" element={<ProjectsPage />} />
              <Route path="projects/:projectId" element={<ProjectDetailPage />} />
              <Route path="projects/:projectId/knowledge" element={<ProjectKnowledgePage />} />
              <Route path="tasks" element={<TasksPage />} />
              <Route path="tasks/:taskId" element={<TaskDetailPage />} />
              <Route path="verification" element={<VerificationPage />} />
              <Route path="recovery" element={<RecoveryPage />} />
              <Route path="data-protection" element={<DataProtectionPage />} />
              <Route path="diagnostics" element={<DiagnosticsPage />} />
              <Route path="persistence" element={<Navigate to="/diagnostics" replace />} />
              <Route path="settings" element={<SettingsPage />} />
              <Route path="*" element={<NotFoundPage />} />
            </Route>
          </Routes>
        </HashRouter>
      </AppProviders>
    </AppErrorBoundary>
  );
}

export default App;
