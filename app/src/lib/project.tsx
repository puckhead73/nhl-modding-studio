import { assetRoot } from "./assetRoot";
// The current open project: id + last-known summary, the on-disk path it was loaded from
// (used by in-place Save), and — for root-folder onboarding — the install report that resolved it
// (databases + save inventory), which downstream features use for save-availability gating.
// ONB-002: when the session was opened from a project folder, `info` is that folder's manifest
// view (destinations, extraction choice, staged outputs); Save and Deploy route through it.
// Shared across the app shell.

import { createContext, useContext, useState, type ReactNode } from "react";
import type { ProjectSummaryDto } from "../bindings/ProjectSummaryDto";
import type { InstallReportDto } from "../bindings/InstallReportDto";
import type { IsoStagedDto } from "../bindings/IsoStagedDto";
import type { ProjectInfoDto } from "../bindings/ProjectInfoDto";
import type { SaveEditorOpenDto } from "../bindings/SaveEditorOpenDto";

type OpenProject = {
  summary: ProjectSummaryDto;
  dbPath: string;
  install: InstallReportDto | null;
  /** When the project was opened from an Xbox 360 disc image: the staging folder it edits. */
  iso: IsoStagedDto | null;
  /** ONB-002: the project folder this session edits, or `null` for the advanced routes. */
  info: ProjectInfoDto | null;
  /** Save Editor: the opened save or database, when this session belongs to the Save Editor. */
  saveFile: SaveEditorOpenDto | null;
};

type ProjectState = {
  project: ProjectSummaryDto | null;
  dbPath: string | null;
  /** The resolved install (root-folder onboarding only); `null` for the legacy file-pick path. */
  install: InstallReportDto | null;
  /** The disc-image staging this project edits (Xbox 360 `.iso` route), else `null`. */
  iso: IsoStagedDto | null;
  /** The project folder (ONB-002), else `null`. */
  info: ProjectInfoDto | null;
  /** Save Editor session details, else `null` (a Mod Studio session). */
  saveFile: SaveEditorOpenDto | null;
  open: (
    summary: ProjectSummaryDto,
    dbPath: string,
    install?: InstallReportDto | null,
    iso?: IsoStagedDto | null,
    info?: ProjectInfoDto | null,
    saveFile?: SaveEditorOpenDto | null,
  ) => void;
  update: (summary: ProjectSummaryDto) => void;
  /** Adopt a fresh manifest view (after a project save, an extraction choice or a deployment). */
  updateInfo: (info: ProjectInfoDto, discovery?: InstallReportDto, projectId?: number) => void;
  close: () => void;
};

const ProjectContext = createContext<ProjectState | null>(null);

export function ProjectProvider({ children }: { children: ReactNode }) {
  const [state, setState] = useState<OpenProject | null>(null);
  const value: ProjectState = {
    project: state?.summary ?? null,
    dbPath: state?.dbPath ?? null,
    install: state?.install ?? null,
    iso: state?.iso ?? null,
    info: state?.info ?? null,
    saveFile: state?.saveFile ?? null,
    open: (summary, dbPath, install = null, iso = null, info = null, saveFile = null) => setState({ summary, dbPath, install, iso, info, saveFile }),
    // The shared publication boundary (LEAGUE-PARITY-COMPLETION-R1 F3): a summary is adopted only
    // for the project that is open *now*. A late response for a project that was closed or
    // replaced while the request was in flight would otherwise pair the new project's paths and
    // install with the old project's summary and id; here it is dropped instead.
    update: (summary) =>
      setState((s) => (s && s.summary.project_id === summary.project_id ? { ...s, summary } : s)),
    updateInfo: (info, discovery, projectId) => setState((s) => (
      s && s.info && s.info.dir === info.dir && (projectId == null || s.summary.project_id === projectId)
        ? { ...s, info, ...(discovery ? { install: discovery } : {}) } : s
    )),
    close: () => setState(null),
  };
  return <ProjectContext.Provider value={value}>{children}</ProjectContext.Provider>;
}

export function useProject(): ProjectState {
  const ctx = useContext(ProjectContext);
  if (!ctx) throw new Error("useProject must be used within a ProjectProvider");
  return ctx;
}

/**
 * `true` when the open session belongs to the Save Editor — a single save or extracted database
 * with no game installation. Surfaces that edit game files (art, audio, localization, game
 * settings) are hidden there; database surfaces stay.
 */
export function useIsSaveEditor(): boolean {
  return useProject().saveFile != null;
}

/**
 * The asset-root arguments every asset command needs — install dir + platform (which drives the
 * on-disk art layout server-side). `installDir` is `null` for the legacy file-pick path (no install
 * tree), which callers use to gate their queries. Shared so the header previews and the asset pages
 * build roots identically.
 */
export function useAssetRootArgs() {
  const { install } = useProject();
  const installDir = assetRoot(install) ?? null;
  const rootArgs = {
    install_dir: installDir ?? "",
    platform: install?.platform ?? undefined,
    rendering_override: null,
    fe_override: null,
  };
  return { installDir, rootArgs };
}
