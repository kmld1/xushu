import { HashRouter, Route, Routes } from "react-router";

import { AppLayout } from "@/app/app-layout";
import { AppProviders } from "@/app/providers";
import { AppErrorBoundary } from "@/components/app-error-boundary";
import { NotFoundPage } from "@/pages/not-found-page";
import { OverviewPage } from "@/pages/overview-page";
import { PersistencePage } from "@/pages/persistence-page";
import { SettingsPage } from "@/pages/settings-page";

function App() {
  return (
    <AppErrorBoundary>
      <AppProviders>
        <HashRouter>
          <Routes>
            <Route element={<AppLayout />}>
              <Route index element={<OverviewPage />} />
              <Route path="persistence" element={<PersistencePage />} />
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
