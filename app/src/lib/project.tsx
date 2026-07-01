// The current open project: id + last-known summary, the on-disk path it was loaded from
// (used by in-place Save), and — for root-folder onboarding — the install report that resolved it
// (databases + save inventory), which downstream features use for save-availability gating.
// Shared across the app shell.

import { createContext, useContext, useState, type ReactNode } from "react";
import type { ProjectSummaryDto } from "../bindings/ProjectSummaryDto";
import type { InstallReportDto } from "../bindings/InstallReportDto";

type OpenProject = {
  summary: ProjectSummaryDto;
  dbPath: string;
  install: InstallReportDto | null;
};

type ProjectState = {
  project: ProjectSummaryDto | null;
  dbPath: string | null;
  /** The resolved install (root-folder onboarding only); `null` for the legacy file-pick path. */
  install: InstallReportDto | null;
  open: (summary: ProjectSummaryDto, dbPath: string, install?: InstallReportDto | null) => void;
  update: (summary: ProjectSummaryDto) => void;
  close: () => void;
};

const ProjectContext = createContext<ProjectState | null>(null);

export function ProjectProvider({ children }: { children: ReactNode }) {
  const [state, setState] = useState<OpenProject | null>(null);
  const value: ProjectState = {
    project: state?.summary ?? null,
    dbPath: state?.dbPath ?? null,
    install: state?.install ?? null,
    open: (summary, dbPath, install = null) => setState({ summary, dbPath, install }),
    update: (summary) => setState((s) => (s ? { ...s, summary } : null)),
    close: () => setState(null),
  };
  return <ProjectContext.Provider value={value}>{children}</ProjectContext.Provider>;
}

export function useProject(): ProjectState {
  const ctx = useContext(ProjectContext);
  if (!ctx) throw new Error("useProject must be used within a ProjectProvider");
  return ctx;
}
