import React from "react";
import ReactDOM from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { RouterProvider } from "react-router-dom";
import { ProjectProvider } from "./lib/project";
import { SettingsProvider } from "./lib/settings";
import { RosterCartProvider } from "./lib/rosterCart";
import { WindowCloseGuard } from "./components/WindowCloseGuard";
import { router } from "./routes";
import "./index.css";

const queryClient = new QueryClient({
  defaultOptions: {
    queries: { staleTime: 1000 * 30, gcTime: 1000 * 60 * 5, retry: false },
  },
});

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <QueryClientProvider client={queryClient}>
      <SettingsProvider>
        <ProjectProvider>
          <WindowCloseGuard />
          <RosterCartProvider>
            <RouterProvider router={router} />
          </RosterCartProvider>
        </ProjectProvider>
      </SettingsProvider>
    </QueryClientProvider>
  </React.StrictMode>,
);
