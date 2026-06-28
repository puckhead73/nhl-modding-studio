// The current open project: id + last-known summary, plus the on-disk path it was loaded from
// (used by in-place Save). Shared across the app shell.

import { createContext, useContext, useState, type ReactNode } from "react";
import type { ProjectSummaryDto } from "../bindings/ProjectSummaryDto";

type OpenProject = { summary: ProjectSummaryDto; dbPath: string };

type ProjectState = {
  project: ProjectSummaryDto | null;
  dbPath: string | null;
  open: (summary: ProjectSummaryDto, dbPath: string) => void;
  update: (summary: ProjectSummaryDto) => void;
  close: () => void;
};

const ProjectContext = createContext<ProjectState | null>(null);

export function ProjectProvider({ children }: { children: ReactNode }) {
  const [state, setState] = useState<OpenProject | null>(null);
  const value: ProjectState = {
    project: state?.summary ?? null,
    dbPath: state?.dbPath ?? null,
    open: (summary, dbPath) => setState({ summary, dbPath }),
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
