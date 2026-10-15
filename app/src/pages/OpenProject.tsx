import { useEffect, useMemo, useRef, useState } from "react";
import { useLocation, useNavigate } from "react-router-dom";
import { useMutation, useQuery } from "@tanstack/react-query";
import { open as openDialog, confirm as confirmDialog } from "@tauri-apps/plugin-dialog";
import { call, type Platform } from "../lib/client";
import type { InstallReportDto } from "../bindings/InstallReportDto";
import type { ProjectOpenDto } from "../bindings/ProjectOpenDto";
import type { ImportReviewDto } from "../bindings/ImportReviewDto";
import type { BaselineTargetDto } from "../bindings/BaselineTargetDto";
import type { ExtractPreflightDto } from "../bindings/ExtractPreflightDto";
import { VanillaCheck } from "../components/VanillaCheck";
import { useProject } from "../lib/project";
import { assetRoot } from "../lib/assetRoot";
import { useSettings } from "../lib/settings";
import { projectLocation, organizedProjectLocation } from "../lib/projectLocation";
import { GameSourcePicker, type GameSource } from "../components/GameSourcePicker";
import { jobFinished } from "../components/ExtractionPanel";
import type { ExtractJobDto } from "../bindings/ExtractJobDto";
import { ProjectLocationFields, ProjectsStorageField } from "../components/ProjectLocationFields";
import { ImportProjectArchive } from "../components/ImportProjectArchive";
import {
  CONSOLE_ICON,
  CONSOLE_LABEL,
  GAMES,
  gameById,
  img,
  type GameDef,
} from "../lib/catalog";
import { Button, ErrorBanner, Pill, Spinner } from "../components/primitives";
import { GamePlatformPill } from "../components/GamePlatformPill";
import { LingeringProjectNotice } from "../components/LingeringProjectNotice";
import { IsoOpenPanel } from "../components/IsoOpenPanel";
import { Ps3DiscPanel } from "../components/Ps3DiscPanel";
import { SourceFolderPanel } from "../components/SourceFolderPanel";
import { Ps4PackagePanel } from "../components/Ps4PackagePanel";
import { ExtractionPanel } from "../components/ExtractionPanel";
import headerLogo from "../assets/ui/NHL Modding Studio Header.png";

/**
 * ONB-002 onboarding: source + project folder → game → console → paths (project folder, game
 * installation folder, save-data location, plus the import sources) → extraction, only when
 * needed → Studio. Every edit output lives in the project folder; the installation and the save
 * location are deployment destinations written only by an explicit deployment (and by the
 * extraction step, which is the one installation-write exception).
 */
type Step = "storage" | "load" | "archive" | "source" | "game" | "platform" | "paths" | "extract" | "resume";
type SourceKind = "fresh" | "import";

/** ONB-003: where Studio lands after opening a project — the last meaningful view when it is still
 *  a Studio route, else the first page the project's mode can show. */
export function landingRoute(opened: ProjectOpenDto): string {
  const last = opened.last_view;
  if (last && last.startsWith("/app/") && !last.includes("..")) return last;
  return opened.project.mode === "assets" ? "/app/assets/players" : "/app/database/teams";
}

const inputCls =
  "w-full rounded-md border border-slate-700 bg-slate-950 px-2 py-1.5 text-sm text-slate-100 placeholder-slate-600 focus:border-sky-600 focus:outline-none";

export function OpenProject() {
  const { open, project: lingering } = useProject();
  const navigate = useNavigate();
  const location = useLocation();
  const { settings, getPathEntry, setPathEntry, rememberProject, forgetProject } = useSettings();

  // Callers can force the picker (skipping the remembered default) via navigation state — e.g. the
  // CorruptionGuard's "Back to start", which must not reopen the same corrupted default and loop.
  const forcePicker = (location.state as { pick?: boolean } | null)?.pick === true;
  const escapeBlocked = forcePicker && lingering != null;

  // The remembered startup default (game + console + paths). ONB-002: it pre-selects the target
  // and pre-fills the path screen; it never opens anything by itself (the old auto-open opened the
  // live install under the old write model).
  const initial = useMemo(() => {
    if (forcePicker) return null;
    const g = gameById(settings.defaultGameId);
    const p = settings.defaultPlatform;
    if (g && p && g.platforms.includes(p)) {
      const entry = getPathEntry(g.id, p);
      return { game: g, platform: p, root: entry.root, savePath: entry.savePath, saveFile: entry.saveFile ?? "" };
    }
    return null;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const deleteRecent = useMutation({
    mutationFn: async (projectDir: string) => {
      const accepted = await confirmDialog("Permanently delete this project and all files in its folder? This cannot be undone.\n\n" + projectDir,
        { title: "Delete project?", kind: "warning", okLabel: "Delete project", cancelLabel: "Cancel" });
      if (!accepted) return false;
      await call("project_delete", { projectDir });
      return true;
    },
    onSuccess: (deleted, path) => { if (deleted) { forgetProject(path); if (existingDir === path) setExistingDir(""); } },
  });

  const [step, setStep] = useState<Step>("source");
  const [sourceKind, setSourceKind] = useState<SourceKind | null>(null);
  const [projectName, setProjectName] = useState("");

  const [game, setGame] = useState<GameDef | null>(initial?.game ?? null);
  const [platform, setPlatform] = useState<Platform | null>(initial?.platform ?? null);
  const locationResult = organizedProjectLocation(settings.projectsRoot, projectName, platform, game?.id);
  const [projectFolderOverride, setProjectFolderOverride] = useState("");
  const projectDir = projectFolderOverride || locationResult.path;
  const projectLocationError = locationResult.error;
  const [gameSource, setGameSource] = useState<GameSource | null>(null);
  const [sourcePickerOpen, setSourcePickerOpen] = useState(false);
  const [discJob, setDiscJob] = useState<ExtractJobDto | null>(null);
  const completedDisc = useRef<string | null>(null);
  const cancelDisc = useMutation({ mutationFn: (job: number) => call("extract_cancel", { job }) });
  const [pendingChoice, setPendingChoice] = useState<"fresh" | "import" | "load" | "archive">("fresh");
  const storageError = projectLocation(settings.projectsRoot, "Builds").error;
  function enterStudio(choice: "fresh" | "import" | "load" | "archive") {
    setPendingChoice(choice);
    if (storageError) { setStep("storage"); return; }
    if (choice === "load" || choice === "archive") setStep(choice);
    else pickSource(choice);
  }

  const [root, setRoot] = useState(initial?.root ?? "");
  const [report, setReport] = useState<InstallReportDto | null>(null);
  const prefilledFor = useRef<string | null>(null);

  // Import sources (Use my own files): read sources, never destinations. ONB-003: any one of the
  // game database or a folder of extracted assets is enough.
  const [srcMainDb, setSrcMainDb] = useState("");
  const [srcDictionary, setSrcDictionary] = useState("");
  const [srcAssets, setSrcAssets] = useState("");
  const [sourceFolderBusy, setSourceFolderBusy] = useState(false);
  const [importReview, setImportReview] = useState<ImportReviewDto | null>(null);
  const importSources = () => ({
    main_db: srcMainDb.trim() || undefined,
    meta: undefined,
    dictionary: srcDictionary.trim() || undefined,
    roster_save: undefined,
    assets_dir: srcAssets.trim() || undefined,
  });
  const anyImportSource = srcMainDb.trim() !== "" || srcAssets.trim() !== "";

  // ONB-003: Open existing project — the folder picked, the opened project and its warnings.
  const [existingDir, setExistingDir] = useState("");
  const [resumed, setResumed] = useState<ProjectOpenDto | null>(null);
  const openExisting = useMutation({
    mutationFn: (dir: string) => call("project_open", { projectDir: dir }),
    onSuccess: (r, dir) => {
      rememberProject(r.project.dir || dir);
      setResumed(r);
      setStep("resume");
    },
  });
  const openImported = useMutation({
    mutationFn: (dir: string) => call("project_open", { projectDir: dir }),
    onSuccess: (r, dir) => {
      rememberProject(r.project.dir || dir);
      // The package already supplies the game's identity and working files.
      finish(r);
    },
  });

  // The created project and the extraction preflight that decides whether screen four is shown.
  const [created, setCreated] = useState<ProjectOpenDto | null>(null);
  const [createdPreflight, setCreatedPreflight] = useState<ExtractPreflightDto | null>(null);
  const [filesReady, setFilesReady] = useState(false);
  const [assetsExtracted, setAssetsExtracted] = useState(false);
  const [assetExtractionBusy, setAssetExtractionBusy] = useState(false);

  const inventory = useQuery({ queryKey: ["baseline_inventory"], queryFn: () => call("baseline_inventory", {}) });
  const baseline: BaselineTargetDto | null =
    game && platform ? (inventory.data?.targets.find((t) => t.game_id === game.id && t.platform === platform) ?? null) : null;

  const detect = useMutation({
    mutationFn: () => call("detect_install", { gameId: game!.id, platform: platform!, root, savePath: null }),
    onSuccess: (r) => setReport(r),
  });

  const readyToDetect =
    sourceKind !== "fresh" && step === "paths" && !!game && !!platform && root.trim() !== "";
  useEffect(() => {
    if (!readyToDetect) return;
    const t = setTimeout(() => detect.mutate(), 400);
    return () => clearTimeout(t);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [readyToDetect, root, platform, step]);

  const reviewImport = useMutation({
    mutationFn: () => call("project_import_review", { gameId: game!.id, platform: platform!, sources: importSources() }),
    onSuccess: setImportReview,
    onError: () => setImportReview(null),
  });
  useEffect(() => {
    if (step !== "paths" || sourceKind !== "import" || !game || !platform || !anyImportSource) return;
    const t = setTimeout(() => reviewImport.mutate(), 400);
    return () => clearTimeout(t);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [step, sourceKind, srcMainDb, srcDictionary, srcAssets, game, platform]);

  // Remember source and deployment paths per target; project storage is a global preference.
  useEffect(() => {
    if (step !== "paths" || !game || !platform) return;
    if (root.trim() === "" && projectDir.trim() === "") return;
    setPathEntry(game.id, platform, { ...getPathEntry(game.id, platform), root });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [step, root, projectDir, game, platform]);

  // Next on the path screen: create the project (fresh or import), then preflight extraction on
  // the resolved installation; only when something needs extracting is screen four shown.
  const create = useMutation({
    mutationFn: async () => {
      if (projectLocationError) throw new Error(projectLocationError);
      let gameRoot = root.trim();
      if (sourceKind === "fresh") {
        if (!gameSource) throw new Error("Select a game source first.");
        gameRoot = gameSource.destination;
        const identity = JSON.stringify(gameSource);
        if (gameSource.kind === "iso" && completedDisc.current !== identity) {
          let job = await call("disc_extract_start", { image: gameSource.path, root: gameRoot });
          setDiscJob(job);
          while (!jobFinished(job)) {
            await new Promise((resolve) => setTimeout(resolve, 500));
            job = await call("extract_status", { job: job.job });
            setDiscJob(job);
          }
          if (job.state !== "done") throw new Error(job.error || "Disc extraction did not complete. You can retry using the same destination.");
          completedDisc.current = identity;
        }
        const detected = await call("detect_install", { gameId: game!.id, platform: platform!, root: gameRoot, savePath: null });
        if (!detected.install_dir && !detected.game_dir) throw new Error("The game source was not recognized. Select the dumped game's root folder.");
      }
      const common = { projectDir: projectDir.trim(), gameId: game!.id, platform: platform!, root: gameRoot, savePath: null, rosterSave: null };
      const opened =
        sourceKind === "fresh"
          ? await call("project_create_fresh", common)
          : await call("project_create_import", { ...common, sources: importSources() });
      // Imported asset folders can still contain packed archives. Use the resolved
      // asset root even when no separate deployment installation was supplied.
      const extractRoot = assetRoot(opened.discovery);
      let pf: ExtractPreflightDto | null = null;
      if (extractRoot) {
        try {
          pf = await call("extract_preflight", { root: extractRoot, platform: platform! });
        } catch {
          pf = null;
        }
      }
      return { opened, pf, extractRoot };
    },
    onSuccess: async ({ opened, pf }) => {
      rememberProject(opened.project.dir);
      if (game && platform === "ps4") {
        setPathEntry(game.id, platform, { ...getPathEntry(game.id, platform), root });
      }
      setCreated(opened);
      setCreatedPreflight(pf);
      setAssetsExtracted(pf?.state === "complete");
      setFilesReady(false);
      if ((game && ["nhl07", "nhl08"].includes(game.id) && platform === "xbox" && assetRoot(opened.discovery)) || (pf && (pf.state === "needed" || pf.state === "partial" || pf.state === "blocked"))) {
        setStep("extract");
      } else {
        let info = opened.project;
        try {
          info = await call("project_set_extraction", { projectId: opened.summary.project_id, choice: pf?.state === "complete" ? "extracted" : "not_needed" });
        } catch {
          // The choice is bookkeeping; opening the project does not depend on it.
        }
        finish({ ...opened, project: info });
      }
    },
  });

  const chooseExtraction = useMutation({
    mutationFn: (choice: "skipped" | "extracted") => call("project_set_extraction", { projectId: created!.summary.project_id, choice }),
    onSuccess: (info) => finish({ ...created!, project: info }),
  });

  function finish(opened: ProjectOpenDto) {
    open(opened.summary, opened.project.working_main_db ?? opened.project.working_roster_save ?? opened.project.working_dir, opened.discovery, null, opened.project);
    navigate(landingRoute(opened));
  }
  async function browseExisting() {
    const picked = await openDialog({ multiple: false, directory: true, title: "Pick the project folder (it holds nhl-studio-project.json)" });
    if (typeof picked === "string") {
      setExistingDir(picked);
      openExisting.mutate(picked);
    }
  }

  function clearScan() {
    setReport(null);

  }
  function changeRoot(next: string) {
    setRoot(next);

    clearScan();
  }
  function loadSavedPaths(g: GameDef, p: Platform) {
    const entry = getPathEntry(g.id, p);
    setRoot(entry.root);


  }
  function pickSource(kind: SourceKind) {
    setSourceKind(kind);
    setStep("game");
  }
  function pickGame(g: GameDef) {
    if (["nhl22", "nhl23", "nhl24"].includes(g.id)) {
      navigate("/frostbite");
      return;
    }
    setGameSource(null); setDiscJob(null); completedDisc.current = null; setProjectFolderOverride("");
    setGame(g);
    clearScan();
    if (g.platforms.length === 1) {
      const p = g.platforms[0];
      setPlatform(p);
      loadSavedPaths(g, p);
      setStep("paths");
    } else {
      setPlatform(null);
      setStep("platform");
    }
  }
  function pickPlatform(p: Platform) {
    setGameSource(null); setDiscJob(null); completedDisc.current = null; setProjectFolderOverride("");
    setPlatform(p);
    clearScan();
    if (game) loadSavedPaths(game, p);
    setStep("paths");
  }
  function resetFlow() {
    setReport(null);
    setRoot("");


    setSrcMainDb("");
    setSrcDictionary("");

    setSrcAssets("");
    setImportReview(null);
    prefilledFor.current = null;
    setStep("game");
  }

  async function browseDir(title: string, set: (p: string) => void) {
    const picked = await openDialog({ multiple: false, directory: true, title });
    if (typeof picked === "string") set(picked);
  }
  async function browseFile(title: string, extensions: string[], set: (p: string) => void) {
    const picked = await openDialog({ multiple: false, directory: false, title, filters: [{ name: "Game file", extensions }] });
    if (typeof picked === "string") set(picked);
  }

  const [packageBusy, setPackageBusy] = useState(false);
  const fresh = sourceKind === "fresh";
  const freshBlocked = fresh && !!baseline && baseline.status === "unavailable";
  const modBaselineGaps = baseline?.gaps.filter((gap) => !gap.component.startsWith("roster_save")) ?? [];
  const importReady = !fresh && !!importReview?.ok;
  // ONB-003: only the project folder and the inputs are mandatory; the installation and the save
  // location are final destinations that may stay empty until a deployment needs them.
  const canNext =
    !!game &&
    !!platform &&
    !projectLocationError &&
    (!fresh || !!gameSource) &&
    !freshBlocked &&
    (fresh || importReady) &&
    !create.isPending &&
    !sourceFolderBusy &&
    !packageBusy &&
    !escapeBlocked;
  const nextReason = escapeBlocked
    ? "The previous project is still open — close it (or return to it) first."
    : packageBusy
      ? "Wait for package extraction to finish, or cancel it before continuing."
    : projectLocationError
      ? projectLocationError
      : freshBlocked
        ? "Start fresh is not available for this title (see the baseline note)."
        : fresh && !gameSource
          ? "Select a game source."
        : !fresh && !importReady
              ? anyImportSource
                ? "Waiting for the review of your files."
                : "Choose at least one of your own files: a game database or a folder of extracted assets."
              : undefined;

  const gameTag = game && platform && (
    <div className="flex items-center gap-2">
      <GamePlatformPill gameId={game.id} platform={platform} />
      <button type="button" onClick={resetFlow} disabled={create.isPending} className="text-[11px] text-sky-400 underline-offset-2 hover:underline">
        ← Change game
      </button>
    </div>
  );

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-slate-950/80 p-6 backdrop-blur-sm">
      {/* Keep every wizard step reachable when detection results exceed the window height. */}
      <div className="max-h-full min-w-0 w-full max-w-3xl overflow-auto rounded-lg border border-slate-800 bg-slate-900 p-6 shadow-2xl">
        <img src={headerLogo} alt="NHL Modding Studio" className="mx-auto mb-5 h-12 w-auto object-contain" />

        {forcePicker && <LingeringProjectNotice />}

        {step === "source" && (
          <>
            <h1 className="mb-5 text-center text-xl font-semibold text-slate-100">Mod Studio</h1>
            <div className="grid gap-3 sm:grid-cols-2">
              <Button onClick={() => enterStudio("fresh")} disabled={escapeBlocked}>Create New Project</Button>
              <Button onClick={() => enterStudio("load")} disabled={escapeBlocked}>Load Project</Button>
              <Button onClick={() => enterStudio("archive")} disabled={escapeBlocked}>Import Project Package</Button>
              <Button onClick={() => enterStudio("import")} disabled={escapeBlocked}>Import non-studio project</Button>
            </div>
            <div className="mt-4"><Button variant="secondary" onClick={() => navigate("/", { state: location.state })}>Back</Button></div>
          </>
        )}
        {step === "storage" && (
          <div className="space-y-4">
            <h1 className="text-xl font-semibold text-slate-100">Choose your default NHL Project folder</h1>
            <ProjectsStorageField />
            <p className="text-sm text-slate-400">Projects are organized by platform, game, then project name. Released builds go in Builds.</p>
            {storageError && <p className="text-xs text-amber-300">{storageError}</p>}
            <Button disabled={!!storageError} onClick={() => enterStudio(pendingChoice)}>Continue</Button>
            <Button variant="secondary" onClick={() => setStep("source")}>Back</Button>
          </div>
        )}
        {step === "archive" && <>
          <h1 className="text-xl font-semibold text-slate-100">Import Project Package</h1>
          <ImportProjectArchive projectsRoot={settings.projectsRoot} disabled={openImported.isPending || escapeBlocked} pickOnMount
            onImported={dir => { setExistingDir(dir); openImported.mutate(dir); }} />
          {openImported.isPending && <p role="status">Opening project…</p>}
          {openImported.isError && <div className="my-3">
            <ErrorBanner title="Project imported, but couldn't open" message={String(openImported.error)} />
            <p className="mt-2 break-all text-sm text-slate-300">Your imported project is saved at {existingDir}.</p>
            <Button onClick={() => openImported.mutate(existingDir)}>Retry opening project</Button>
          </div>}
          <Button variant="secondary" disabled={openImported.isPending} onClick={() => setStep("source")}>Back</Button>
        </>}
        {step === "load" && (
          <>
            <h1 className="text-xl font-semibold text-slate-100">Load Project</h1>
            <ImportProjectArchive projectsRoot={settings.projectsRoot} disabled={openImported.isPending || escapeBlocked} onImported={dir => { setExistingDir(dir); openImported.mutate(dir); }} />
            {openImported.isError && <ErrorBanner title="Couldn't open the imported project" message={`${existingDir}: ${String(openImported.error)}`} />}
            <Button variant="secondary" onClick={() => void browseExisting()}>Browse for project…</Button>
            <div className="mt-3 rounded-md border border-slate-800 bg-slate-950/40 p-3" data-testid="open-existing">
              <label className="mb-1 block text-xs font-medium text-slate-300">Open existing project — folder</label>
              <div className="flex gap-2">
                <input
                  aria-label="Existing project folder"
                  className={inputCls + " flex-1 font-mono text-xs"}
                  value={existingDir}
                  onChange={(e) => setExistingDir(e.target.value)}
                  placeholder="A folder holding nhl-studio-project.json"
                />
                <Button variant="secondary" onClick={() => openExisting.mutate(existingDir.trim())} disabled={existingDir.trim() === "" || openExisting.isPending || escapeBlocked}>
                  {openExisting.isPending ? "Opening…" : "Open"}
                </Button>
              </div>
              {settings.recentProjects.length > 0 && (
                <ul className="mt-2 space-y-1 text-xs" aria-label="Recent projects">
                  {settings.recentProjects.map((p) => (
                    <li key={p} className="flex items-center gap-2">
                      <button type="button" onClick={() => { setExistingDir(p); openExisting.mutate(p); }} disabled={openExisting.isPending || deleteRecent.isPending || escapeBlocked} className="truncate font-mono text-sky-400 underline-offset-2 hover:underline disabled:opacity-50">
                        {p}
                      </button>
                      <button type="button" onClick={() => forgetProject(p)} disabled={deleteRecent.isPending} className="text-[11px] text-slate-500 hover:text-slate-300" title="Remove from the recent list (the folder is not touched)">
                        Forget
                      </button>
                      <button type="button" onClick={() => deleteRecent.mutate(p)} disabled={deleteRecent.isPending || openExisting.isPending || openImported.isPending || escapeBlocked} className="text-[11px] text-rose-400 hover:text-rose-300 disabled:opacity-50" title="Delete the project folder and its files">
                        Delete
                      </button>
                    </li>
                  ))}
                </ul>
              )}
              {deleteRecent.isError && <ErrorBanner title="Couldn’t delete the project" message={String(deleteRecent.error)} />}
              {openExisting.isError && (
                <div className="mt-2">
                  <ErrorBanner title="Couldn't open that project" message={String(openExisting.error)} />
                </div>
              )}
            </div>

            <Button variant="secondary" onClick={() => setStep("source")}>Back</Button>
          </>
        )}

        {step === "game" && (
          <>
            <header className="mb-5 text-center">
              <h1 className="text-xl font-semibold text-slate-100">Select Game</h1>
              <p className="mt-1 text-xs text-slate-400">Which NHL title are you modding?</p>
            </header>
            <Button variant="secondary" onClick={() => setStep("source")}>Back</Button>
            <div className="mt-4 grid grid-cols-3 gap-3 sm:grid-cols-4">
              {GAMES.map((g) => (
                <button
                  key={g.id}
                  type="button"
                  onClick={() => pickGame(g)}
                  className="group flex cursor-pointer flex-col items-center gap-2 rounded-lg border border-slate-700 bg-slate-950 p-3 transition-colors hover:border-sky-500"
                >
                  {img(g.file) ? (
                    <img
                      src={img(g.file)}
                      alt={g.label}
                      draggable={false}
                      className="h-24 w-auto max-w-full select-none rounded-xl object-contain transition-transform duration-200 ease-out group-hover:scale-105"
                    />
                  ) : (
                    <div className="flex h-24 w-16 items-center justify-center rounded-xl border border-dashed border-slate-700 text-center text-[10px] text-slate-500">
                      {g.label}
                    </div>
                  )}
                  <span className="text-center text-xs font-semibold text-slate-100">{g.label}</span>
                  {g.inDevelopment && (
                    <span className="rounded bg-amber-500/15 px-1.5 py-0.5 text-[10px] font-medium text-amber-300" title="Support for this title is still being added; opening its files may not work yet.">
                      In development
                    </span>
                  )}
                </button>
              ))}
            </div>
            {!sourceKind && (
              <p className="mt-4 text-center text-[11px] text-amber-300">
                <button type="button" onClick={() => setStep("source")} className="underline-offset-2 hover:underline">
                  ← Choose Create New Project or Import non-studio project first
                </button>
              </p>
            )}
          </>
        )}

        {step === "platform" && game && (
          <>
            <header className="mb-5 text-center">
              <h1 className="text-xl font-semibold text-slate-100">Select Console</h1>
              <p className="mt-1 text-xs text-slate-400">
                {game.label} ·{" "}
                <button type="button" className="underline hover:text-slate-200" onClick={() => setStep("game")}>
                  Change game
                </button>
              </p>
            </header>
            <div className="mx-auto grid max-w-md grid-cols-2 gap-4">
              {game.platforms.map((p) => {
                const icon = CONSOLE_ICON[p] ? img(CONSOLE_ICON[p]!) : "";
                return (
                  <button
                    key={p}
                    type="button"
                    onClick={() => pickPlatform(p)}
                    className="group flex cursor-pointer flex-col items-center gap-2 rounded-lg border border-slate-700 bg-slate-950 p-4 transition-colors hover:border-sky-500"
                  >
                    {icon ? (
                      <img src={icon} alt={CONSOLE_LABEL[p]} draggable={false} className="h-28 w-auto max-w-full select-none object-contain transition-transform duration-200 ease-out group-hover:scale-105" />
                    ) : (
                      <div className="flex h-28 w-28 items-center justify-center rounded-lg border border-dashed border-slate-600 text-lg font-bold text-slate-300 transition-transform duration-200 ease-out group-hover:scale-105">
                        {CONSOLE_LABEL[p]}
                      </div>
                    )}
                    <span className="text-sm font-semibold text-slate-100">{CONSOLE_LABEL[p]}</span>
                  </button>
                );
              })}
            </div>
          </>
        )}

        {step === "paths" && game && platform && (
          <>
            <header className="mb-4 flex items-center justify-between gap-2">
              <h1 className="text-xl font-semibold text-slate-100">Project Setup</h1>
              {gameTag}
            </header>
            {fresh && baseline?.status === "unavailable" && (
              <div
                className={
                  "mb-3 rounded-md border px-3 py-2 text-xs " +
                  (baseline.status === "unavailable"
                    ? "border-rose-500/30 bg-rose-500/10 text-rose-300"
                    : baseline.status === "partial"
                      ? "border-amber-500/30 bg-amber-500/10 text-amber-300"
                      : "border-emerald-600/40 bg-emerald-500/10 text-emerald-300")
                }
                data-testid="baseline-note"
              >
                <div className="font-semibold">
                  Bundled baseline: not available
                </div>
                {baseline.gaps.length > 0 && (
                  <ul className="mt-1 list-disc pl-4">
                    {baseline.gaps.map((g) => (
                      <li key={g.component}>
                        <span className="font-medium">{g.component.replace(/_/g, " ")}:</span> {g.reason}
                      </li>
                    ))}
                  </ul>
                )}
              </div>
            )}

            <fieldset disabled={create.isPending} className="space-y-3">
              <ProjectLocationFields name={projectName} onNameChange={setProjectName} platform={platform} game={game?.id} folder={projectFolderOverride} onFolderChange={setProjectFolderOverride} />
              {!fresh && <>
              <PathField
                label="Game source folder (optional)"
                ariaLabel="Game source folder"
                value={root}
                onChange={changeRoot}
                placeholder={rootPlaceholder(platform)}
                onBrowse={() => void browseDir(rootBrowseTitle(platform), changeRoot)}
                hint="Studio reads game files from this folder. Mod Manager chooses where your built mod is installed."
                extra={
                  <Button onClick={() => detect.mutate()} disabled={!root.trim() || detect.isPending}>
                    {detect.isPending ? "Scanning…" : "Detect"}
                  </Button>
                }
              />
              </>}
            </fieldset>

            {fresh && <fieldset disabled={create.isPending} className="mt-4 space-y-3">
              {!gameSource ? <Button onClick={() => setSourcePickerOpen(true)}>Select Game Source</Button> :
                <div className="space-y-2 rounded-lg border border-slate-700 bg-slate-950 p-4" data-testid="game-source-card">
                  <h2 className="font-semibold text-slate-100">Game source</h2>
                  {gameSource.kind === "iso" && <p className="break-all text-sm text-slate-300">ISO: {gameSource.path}</p>}
                  <p className="break-all text-sm text-slate-300">{gameSource.kind === "iso" ? "Extraction destination" : "Dumped ISO folder"}: {gameSource.destination}</p>
                  <Button variant="secondary" onClick={() => setSourcePickerOpen(true)}>Change game source</Button>
                </div>}
              {baseline?.status === "partial" && modBaselineGaps.length > 0 && <details className="text-xs text-slate-400"><summary>Baseline availability notes</summary>
                <ul>{modBaselineGaps.map((gap) => <li key={gap.component}>{gap.reason}</li>)}</ul>
              </details>}
            </fieldset>}
            {sourcePickerOpen && platform && <GameSourcePicker platform={platform} value={gameSource} onClose={() => setSourcePickerOpen(false)} onSelect={(source) => {
              create.reset(); cancelDisc.reset(); setGameSource(source); setRoot(source.destination); setReport(null); setDiscJob(null); setSourcePickerOpen(false);
            }} />}
            {discJob && <p role="status" className="mt-3 text-sm text-slate-300">Disc extraction: {discJob.state} — {discJob.current}</p>}
            {discJob && !jobFinished(discJob) && <Button variant="secondary" disabled={cancelDisc.isPending} onClick={() => cancelDisc.mutate(discJob.job)}>Cancel extraction</Button>}
            {cancelDisc.isError && <p role="alert" className="text-rose-300">{String(cancelDisc.error)}</p>}

            {sourceKind === "import" && (
              <div className="mt-4 rounded-md border border-slate-800 bg-slate-950/40 p-3" data-testid="import-sources">
                <div className="mb-2 text-xs font-semibold text-slate-200">Your files (read sources — copied into the project, never written). Any one of them is enough.</div>
                <div className="space-y-2">
                  <PathField
                    label={`Main database (${platform === "ps4" ? "nhl_game.db" : game.id === "nhl07" ? "NHL07NG.db" : "nhlng.db"}, optional) — its -meta.xml beside it, or the bundled schema is used`}
                    ariaLabel="Source main database"
                    value={srcMainDb}
                    onChange={setSrcMainDb}
                    placeholder="…/nhlng.db"
                    onBrowse={() => void browseFile("Pick the main database", ["db"], setSrcMainDb)}
                  />
                  <PathField
                    label="Localization dictionary (optional)"
                    ariaLabel="Source dictionary"
                    value={srcDictionary}
                    onChange={setSrcDictionary}
                    placeholder={platform === "ps4" ? "…/eng_us.db" : "…/nhl_eng_us.db"}
                    onBrowse={() => void browseFile("Pick the localization dictionary", ["db"], setSrcDictionary)}
                  />
                  <PathField
                    label="Game assets folder (packed or extracted, optional)"
                    disabled={sourceFolderBusy}
                    ariaLabel="Source assets folder"
                    value={srcAssets}
                    onChange={setSrcAssets}
                    placeholder="A dumped game folder with .big archives, or extracted rendering/, fe/, …"
                    onBrowse={() => void browseDir("Pick the folder of packed or extracted game assets", setSrcAssets)}
                    hint="Source files for asset editing. Your changes are saved in the project and packaged when you build."
                  />
                  {srcAssets.trim() && <SourceFolderPanel key={`${game.id}:${platform}:${srcAssets}`} root={srcAssets.trim()} gameId={game.id} platform={platform}
                    onBusyChange={setSourceFolderBusy} onReady={(found) => {
                      setSrcMainDb(found.nhlng_path ?? "");
                      if (found.localization_path) setSrcDictionary(found.localization_path);
                      // PS4 install_dir points at t3db; textures live under the game root.
                      setSrcAssets((platform === "ps4" ? found.game_dir : null) ?? found.install_dir ?? srcAssets);
                      setImportReview(null);
                    }} />}
                </div>
                {reviewImport.isPending && <p className="mt-2 text-[11px] text-slate-500">Checking the files…</p>}
                {reviewImport.isError && <p className="mt-2 text-xs text-rose-300">{String(reviewImport.error)}</p>}
                {importReview && (
                  <ul className="mt-2 space-y-1 text-xs" aria-label="Import review">
                    {importReview.items.map((i) => (
                      <li key={i.role} className="flex items-start gap-2">
                        <span className={i.ok ? "text-emerald-400" : "text-rose-400"}>{i.ok ? "✓" : "✗"}</span>
                        <span className="text-slate-300">{i.role.replace(/_/g, " ")}</span>
                        <span className={i.ok ? "text-slate-500" : "text-rose-300"}>{i.detail}</span>
                      </li>
                    ))}
                  </ul>
                )}
              </div>
            )}

            {!fresh && detect.isError && (
              <div className="mt-4">
                <ErrorBanner title="Couldn't scan that folder" message={String(detect.error)} />
              </div>
            )}

            {!fresh && report && (
              <InstallSummary
                report={report}
                onUseGame={changeRoot}
              />
            )}

            {platform === "ps3" && sourceKind === "import" && (
              <Ps3DiscPanel key={game.id} gameId={game.id} onReady={(disc) => {
                setSrcMainDb(disc.nhlng_path ?? "");
                setSrcDictionary(disc.localization_path ?? "");
                setSrcAssets(disc.install_dir ?? "");
                setImportReview(null);
              }} />
            )}
            {platform === "ps4" && sourceKind === "import" && (
              <Ps4PackagePanel key={game.id} gameId={game.id} onBusyChange={setPackageBusy} onReady={(found) => {
                setSrcMainDb(found.nhlng_path ?? "");
                setSrcDictionary(found.localization_path ?? "");
                setSrcAssets(found.game_dir ?? found.install_dir ?? "");
                setImportReview(null);
              }} />
            )}
            {!fresh && platform === "xbox" && (
              <IsoOpenPanel
                gameId={game.id}
                platform={platform}
                defaultStagingRoot={projectDir.trim() ? projectDir.replace(/[\/]+$/, "") + "/iso-staging" : root.trim() ? root.replace(/[\/]+$/, "") + "-studio-staging" : ""}
                onOpened={(staged, summary, install) => {
                  open(summary, staged.db_path, install, staged);
                  navigate("/app/database/teams");
                }}
              />
            )}

            <div className="mt-5 flex items-center justify-end gap-3">
              <Button onClick={() => create.mutate()} disabled={!canNext} title={nextReason}>
                {create.isPending ? "Creating project…" : "Next"}
              </Button>
            </div>
            {nextReason && !create.isPending && <p className="mt-1 text-right text-[11px] text-slate-500">{nextReason}</p>}
            {create.isError && (
              <div className="mt-4">
                <ErrorBanner title="Couldn't create the project" message={String(create.error)} />
              </div>
            )}
          </>
        )}

        {step === "resume" && resumed && (
          <>
            <header className="mb-4 flex items-center justify-between gap-2">
              <h1 className="text-xl font-semibold text-slate-100">Resume project</h1>
              <GamePlatformPill gameId={resumed.project.game_id} platform={resumed.project.platform} />
            </header>
            <p className="mb-3 text-xs text-slate-400">
              <span className="font-mono text-slate-300">{resumed.project.dir}</span> · {resumed.project.mode === "assets" ? "assets only" : resumed.project.mode === "roster" ? "roster save" : "database"} · {resumed.project.saves} save{resumed.project.saves === 1 ? "" : "s"}
              {resumed.project.last_saved ? ` (last ${resumed.project.last_saved})` : ""} · {resumed.project.staged_files.length} staged asset file(s)
            </p>
            <div className="mb-3 text-xs text-slate-400">
              Game source: <span className="font-mono text-slate-300">{resumed.project.destinations.picked_root || "— not linked"}</span>
            </div>
            {resumed.warnings.length > 0 && (
              <ul className="mb-3 space-y-1" aria-label="Resume notes" data-testid="resume-warnings">
                {resumed.warnings.map((w, i) => (
                  <li key={i} className="rounded-md border border-amber-500/30 bg-amber-500/10 px-2 py-1 text-xs text-amber-200">
                    {w}
                  </li>
                ))}
              </ul>
            )}
            <div className="flex items-center justify-end gap-2">
              <Button onClick={() => finish(resumed)}>{resumed.last_view ? "Continue where I left off" : "Continue to Studio"}</Button>
            </div>
          </>
        )}

        {step === "extract" && created && platform && (
          <>
            <header className="mb-4 flex items-center justify-between gap-2">
              <h1 className="text-xl font-semibold text-slate-100">Extract game assets now?</h1>
              {gameTag}
            </header>
            <p className="mb-3 text-xs text-slate-400">
              Your project is ready. Prepare game assets now, or continue and do this later.
            </p>
            <VanillaCheck key={`${created.project.dir}:${created.project.destinations.game_dir ?? created.project.destinations.install_dir ?? root}`} root={created.project.destinations.game_dir ?? created.project.destinations.install_dir ?? root} gameId={game!.id} platform={platform} onReady={setFilesReady} />
            <ExtractionPanel
              projectId={created.summary.project_id}
              onRelink={linked => setCreated(previous => previous ? { ...previous, project: linked.project, discovery: linked.discovery } : previous)}
              root={created.project.destinations.game_dir ?? created.project.destinations.install_dir ?? root}
              platform={platform}
              initialPreflight={createdPreflight ?? undefined}
              mode="onboarding"
              verificationPending={!filesReady}
              onBusyChange={setAssetExtractionBusy}
              onDone={(_job, scan) => setAssetsExtracted(scan.state === "complete")}
              onSkip={() => chooseExtraction.mutate("skipped")}
            />
            <div className="mt-4 flex items-center justify-end gap-2">
              <Button onClick={() => chooseExtraction.mutate(assetsExtracted ? "extracted" : "skipped")} disabled={chooseExtraction.isPending || !filesReady || assetExtractionBusy}>
                Continue to Studio
              </Button>
            </div>
            {chooseExtraction.isError && <p className="mt-2 text-xs text-rose-300">{String(chooseExtraction.error)}</p>}
          </>
        )}

        <p className="mt-5 border-t border-slate-800 pt-3 text-center text-[11px] text-amber-300/70">
          Beta — built and verified from NHL 07 through NHL Legacy. PS4 titles are still a work in progress.
        </p>
      </div>
    </div>
  );
}

function PathField({
  label,
  ariaLabel,
  value,
  onChange,
  placeholder,
  onBrowse,
  hint,
  extra,
  disabled,
}: {
  label: string;
  ariaLabel?: string;
  value: string;
  onChange: (v: string) => void;
  placeholder: string;
  onBrowse: () => void;
  hint?: string;
  extra?: React.ReactNode;
  disabled?: boolean;
}) {
  return (
    <div>
      <label className="mb-1 block text-xs font-medium text-slate-300">{label}</label>
      <div className="flex gap-2">
        <input disabled={disabled} aria-label={ariaLabel ?? label} className={inputCls + " flex-1 font-mono text-xs"} value={value} onChange={(e) => onChange(e.target.value)} placeholder={placeholder} />
        <Button disabled={disabled} variant="secondary" onClick={onBrowse}>
          Browse…
        </Button>
        {extra}
      </div>
      {hint && <p className="mt-1 text-[11px] text-slate-500">{hint}</p>}
    </div>
  );
}

/** What discovery resolved from the chosen root: databases, save inventory, and the modding-minimum gate. */
function InstallSummary({ report, onUseGame }: { report: InstallReportDto; onUseGame: (path: string) => void }) {
  const ps4 = report.platform === "ps4";
  const ambiguous = report.problems.some((p) => p.kind === "game_ambiguous");
  return (
    <div className="mt-4 rounded-md border border-slate-800 bg-slate-950/40 p-3">
      <div className="mb-2 flex items-center gap-2">
        <span className="text-xs font-semibold text-slate-200">Detected in the game source</span>
        {report.has_existing_profile && <Pill tone="info">Older Studio settings file present (read only)</Pill>}
      </div>

      {ps4 && report.problems.length > 0 && (
        <ul className="mb-3 space-y-1" aria-label="Problems">
          {report.problems.map((p, i) => (
            <li
              key={i}
              data-kind={p.kind}
              className={"rounded-md border px-2 py-1 text-xs " + (p.kind.startsWith("save") ? "border-amber-500/30 bg-amber-500/10 text-amber-300" : "border-rose-500/30 bg-rose-500/10 text-rose-300")}
            >
              {p.message}
            </li>
          ))}
        </ul>
      )}

      {ps4 && report.game_candidates.length > 0 && (
        <div className="mb-3">
          <div className="mb-1 text-xs font-semibold text-slate-200">Game folders</div>
          <ul className="space-y-1 text-xs">
            {report.game_candidates.map((c) => (
              <li key={c.path} className="flex items-center gap-2">
                <span className={c.matches ? "text-emerald-400" : "text-slate-500"}>{c.matches ? "✓" : "—"}</span>
                <span className="text-slate-300">
                  {c.title ?? "Unidentified"}
                  {c.title_id && ` · ${c.title_id}`}
                  {c.content_id && ` · ${regionOf(c.content_id)}`}
                  {c.app_version && ` · v${c.app_version}`}
                </span>
                <span className="truncate font-mono text-[11px] text-slate-500" title={c.reason ?? c.path}>
                  {c.path}
                </span>
                {c.matches && ambiguous && (
                  <button type="button" onClick={() => onUseGame(c.path)} className="shrink-0 text-[11px] text-sky-400 underline-offset-2 hover:underline">
                    Use this folder
                  </button>
                )}
              </li>
            ))}
          </ul>
        </div>
      )}

      <ul className="space-y-1 text-xs">
        <FileRow label={report.platform === "ps4" ? "nhl_game.db (source)" : "nhlng.db (source)"} path={report.nhlng_path} optional />
        <FileRow label="metadata" path={report.meta_xml_path} optional />
        <FileRow label="localization" path={report.localization_path} optional />
      </ul>

    </div>
  );
}

// Platform-specific copy for the path step.
function rootBrowseTitle(p: Platform | null): string {
  if (p === "ps3") return "Pick the PS3 disc folder, USRDIR, RPCS3 folder or virtual HDD";
  if (p === "ps4") return "Pick the PS4 game folder (or a folder holding your PS4 games)";
  if (p === "ps5") return "Pick the PS5 game folder (or a folder holding your PS5 games)";
  if (p === "recomp") return "Pick the recompiled game folder";
  return "Pick the game's install folder (contains default.xex)";
}
function rootPlaceholder(p: Platform | null): string {
  if (p === "ps3") return "e.g. D:/PS3 Games/NHL 08/PS3_GAME/USRDIR";
  if (p === "ps4") return "e.g. D:/PS4 Games/CUSA18182";
  if (p === "ps5") return "e.g. D:/PS5 Games/NHL 25";
  if (p === "recomp") return "e.g. C:/Games/NHL Legacy Recompiled";
  return "e.g. C:/Games/NHL14";
}
/** The region from a PS4 content id's prefix (`UP…` → North America, `EP…` → Europe). */
function regionOf(contentId: string): string {
  const prefix = contentId.slice(0, 2).toUpperCase();
  if (prefix === "UP") return "North America";
  if (prefix === "EP") return "Europe";
  if (prefix === "JP") return "Japan";
  if (prefix === "HP") return "Asia";
  return contentId.split("-")[0];
}

function FileRow({ label, path, optional }: { label: string; path: string | null; optional?: boolean }) {
  const ok = !!path;
  return (
    <li className="flex items-center gap-2">
      <span className={ok ? "text-emerald-400" : optional ? "text-slate-500" : "text-rose-400"}>{ok ? "✓" : optional ? "—" : "✗"}</span>
      <span className="text-slate-300">{label}</span>
      {path && <span className="truncate font-mono text-[11px] text-slate-500">{path}</span>}
    </li>
  );
}

// Keep the spinner import used for the remembered-default banner on slow inventories.
export function InventoryPending() {
  return <Spinner label="Reading the bundled baselines…" />;
}
