import { useEffect, useMemo, useRef, useState } from "react";
import { useLocation, useNavigate } from "react-router-dom";
import { useMutation } from "@tanstack/react-query";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { call, type Platform } from "../lib/client";
import type { InstallReportDto } from "../bindings/InstallReportDto";
import type { ExtractSummaryDto } from "../bindings/ExtractSummaryDto";
import { useProject } from "../lib/project";
import { useSettings } from "../lib/settings";
import {
  CONSOLE_ICON,
  CONSOLE_LABEL,
  GAMES,
  gameById,
  img,
  needsSavePath,
  type GameDef,
} from "../lib/catalog";
import { Button, ErrorBanner, Pill, Spinner } from "../components/primitives";
import headerLogo from "../assets/ui/NHL Modding Studio Header.png";

type Step = "game" | "platform" | "root";

const inputCls =
  "w-full rounded-md border border-slate-700 bg-slate-950 px-2 py-1.5 text-sm text-slate-100 placeholder-slate-600 focus:border-sky-600 focus:outline-none";

export function OpenProject() {
  const { open, close } = useProject();
  const navigate = useNavigate();
  const location = useLocation();
  const { settings, getPathEntry, setDefaultTarget, setPathEntry } = useSettings();

  // Callers can force the picker (skipping startup auto-open) via navigation state — e.g. the
  // CorruptionGuard's "Back to start", which must not reopen the same corrupted default and loop.
  const forcePicker = (location.state as { pick?: boolean } | null)?.pick === true;

  // When we were sent here to escape a corrupted project, release that still-open workspace now that
  // we're safely on the picker route (AppShell is unmounted, so nulling the project can't trigger its
  // fallback redirect). CorruptionGuard intentionally leaves the close to us to avoid that clobber.
  useEffect(() => {
    if (forcePicker) close();
    // Run once on mount; `close` is a stable action and `forcePicker` is fixed for this navigation.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Resolve the saved default-startup target once, at first render: if a valid game+platform is
  // configured, skip the picker and land on the folder step with its saved paths pre-filled.
  const initial = useMemo(() => {
    if (forcePicker) return null;
    const g = gameById(settings.defaultGameId);
    const p = settings.defaultPlatform;
    if (g && p && g.platforms.includes(p)) {
      const entry = getPathEntry(g.id, p);
      return { game: g, platform: p, root: entry.root, savePath: entry.savePath };
    }
    return null;
    // Read once on mount — later Settings edits shouldn't yank the in-progress onboarding around.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const [step, setStep] = useState<Step>(initial ? "root" : "game");
  const [game, setGame] = useState<GameDef | null>(initial?.game ?? null);
  const [platform, setPlatform] = useState<Platform | null>(initial?.platform ?? null);

  // Root-folder onboarding.
  const [root, setRoot] = useState(initial?.root ?? "");
  // The save location for platforms without a consistent emulator save path (Xbox / Recomp).
  const [savePath, setSavePath] = useState(initial?.savePath ?? "");
  const [report, setReport] = useState<InstallReportDto | null>(null);
  const [extractResult, setExtractResult] = useState<ExtractSummaryDto | null>(null);

  // Is the current selection the saved startup default?
  const isDefault =
    !!game && settings.defaultGameId === game.id && settings.defaultPlatform === platform;

  // Startup auto-open: when a default is configured we try to open it outright (detect → open) instead
  // of stopping at the folder screen. Any snag — failed scan, unmet modding minimum, un-extracted big
  // files, open error — drops the user back to the game picker with a note.
  const [autoOpening, setAutoOpening] = useState<boolean>(!!initial);
  const [autoOpenError, setAutoOpenError] = useState<string | null>(null);
  const autoOpenStarted = useRef(false);

  const savePathArg = () => (needsSavePath(platform) ? savePath.trim() || null : null);

  const detect = useMutation({
    mutationFn: () =>
      call("detect_install", {
        gameId: game!.id,
        platform: platform!,
        root,
        savePath: savePathArg(),
      }),
    onSuccess: (r) => setReport(r),
  });

  // Extract the game's .big archives in place, then re-detect so the now-loose db is picked up.
  const extract = useMutation({
    mutationFn: () => call("extract_big_files", { gameId: game!.id, platform: platform!, root }),
    onSuccess: (summary) => {
      setExtractResult(summary);
      detect.mutate();
    },
  });

  const openWorkspace = useMutation({
    mutationFn: () =>
      call("open_workspace", {
        gameId: game!.id,
        platform: platform!,
        root,
        savePath: savePathArg(),
        // We only reach "Open" once the db is loose (extraction ran or wasn't needed).
        bigFilesExtracted: true,
      }),
    onSuccess: (summary) => {
      open(summary, report?.nhlng_path ?? root, report);
      navigate("/app/dashboard");
    },
  });

  // Auto-scan once every required path is filled in — the root, plus the save location on platforms
  // that need one (Xbox / Recomp). Entering the save path used to only clear the prior scan, forcing a
  // manual "Detect" re-click; now it re-detects on its own. Debounced so typing/pasting a path doesn't
  // fire a scan per keystroke.
  const readyToDetect =
    !autoOpening &&
    step === "root" &&
    !!game &&
    !!platform &&
    root.trim() !== "" &&
    (!needsSavePath(platform) || savePath.trim() !== "");
  useEffect(() => {
    if (!readyToDetect) return;
    const t = setTimeout(() => detect.mutate(), 400);
    return () => clearTimeout(t);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [readyToDetect, root, savePath, platform, step]);

  // Drop out of the auto-open attempt and show the game picker, optionally with a reason.
  function bailToPicker(reason: string | null) {
    setAutoOpening(false);
    setAutoOpenError(reason);
    setStep("game");
  }

  // Startup auto-open: scan the saved default target and, if it's ready, open the workspace outright.
  // Runs once (the ref guards against StrictMode's double-invoke and any re-render).
  useEffect(() => {
    if (!initial || autoOpenStarted.current) return;
    autoOpenStarted.current = true;
    const { game: g, platform: p, root: r, savePath: sp } = initial;
    const save = needsSavePath(p) ? sp.trim() || null : null;
    (async () => {
      try {
        const report = await call("detect_install", { gameId: g.id, platform: p, root: r, savePath: save });
        if (report.extract_needed) {
          bailToPicker(`${g.label}'s big files need extracting first — open it from here.`);
          return;
        }
        if (!report.minimum_met) {
          bailToPicker(`${g.label}'s nhlng.db and a roster save weren't both found.`);
          return;
        }
        const summary = await call("open_workspace", {
          gameId: g.id,
          platform: p,
          root: r,
          savePath: save,
          bigFilesExtracted: true,
        });
        open(summary, report.nhlng_path ?? r, report);
        navigate("/app/dashboard");
      } catch (e) {
        bailToPicker(`${g.label} failed to open: ${String(e)}`);
      }
    })();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // While this selection is the startup default, keep its saved paths in step with what's typed here,
  // so the next launch pre-fills exactly what the user last used.
  useEffect(() => {
    if (isDefault && game && platform) {
      setPathEntry(game.id, platform, { root, savePath });
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isDefault, root, savePath, game, platform]);

  // Checkbox: make (or clear) this game+platform the Studio's startup default, seeding its saved paths.
  function toggleDefault(checked: boolean) {
    if (!game || !platform) return;
    if (checked) {
      setDefaultTarget(game.id, platform);
      setPathEntry(game.id, platform, { root, savePath });
    } else if (isDefault) {
      setDefaultTarget(null, null);
    }
  }

  function resetFlow() {
    setReport(null);
    setExtractResult(null);
    setRoot("");
    setSavePath("");
    setStep("game");
  }

  function clearScan() {
    setReport(null);
    setExtractResult(null);
  }

  // Load any saved paths for a (game, platform) into the folder fields as the user lands on that step.
  function loadSavedPaths(g: GameDef, p: Platform) {
    const entry = getPathEntry(g.id, p);
    setRoot(entry.root);
    setSavePath(entry.savePath);
  }

  function pickGame(g: GameDef) {
    setGame(g);
    clearScan();
    if (g.platforms.length === 1) {
      const p = g.platforms[0];
      setPlatform(p);
      loadSavedPaths(g, p);
      setStep("root");
    } else {
      setPlatform(null);
      setStep("platform");
    }
  }

  function pickPlatform(p: Platform) {
    setPlatform(p);
    clearScan();
    if (game) loadSavedPaths(game, p);
    setStep("root");
  }

  async function browseRoot() {
    const picked = await openDialog({
      multiple: false,
      directory: true,
      title: rootBrowseTitle(platform),
    });
    if (typeof picked === "string") {
      setRoot(picked);
      clearScan();
    }
  }

  async function browseSave() {
    const picked = await openDialog({
      multiple: false,
      directory: true,
      title: "Pick the folder holding your save",
    });
    if (typeof picked === "string") {
      setSavePath(picked);
      clearScan();
    }
  }

  const gameTag = game && platform && (
    <div className="flex items-center gap-2">
      <Pill tone="info">
        {game.label} · {CONSOLE_LABEL[platform]}
      </Pill>
      <button
        type="button"
        onClick={resetFlow}
        className="text-[11px] text-sky-400 underline-offset-2 hover:underline"
      >
        ← Change game
      </button>
    </div>
  );

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-slate-950/80 p-6 backdrop-blur-sm">
      <div className="w-full max-w-3xl rounded-lg border border-slate-800 bg-slate-900 p-6 shadow-2xl">
        <img src={headerLogo} alt="NHL Modding Studio" className="mx-auto mb-5 h-12 w-auto object-contain" />

        {autoOpening && (
          <div className="flex flex-col items-center gap-4 py-12">
            {initial && img(initial.game.file) ? (
              <img
                src={img(initial.game.file)}
                alt={initial.game.label}
                draggable={false}
                className="h-28 w-auto select-none rounded-xl object-contain"
              />
            ) : null}
            <Spinner label={`Opening ${initial?.game.label ?? "your project"}…`} />
            <button
              type="button"
              onClick={() => bailToPicker(null)}
              className="text-[11px] text-sky-400 underline-offset-2 hover:underline"
            >
              Choose a different game
            </button>
          </div>
        )}

        {autoOpenError && !autoOpening && (
          <div className="mb-5">
            <ErrorBanner title="Couldn't auto-open your default" message={autoOpenError} />
          </div>
        )}

        {!autoOpening && step === "game" && (
          <>
            <header className="mb-5 text-center">
              <h1 className="text-xl font-semibold text-slate-100">Select Game</h1>
              <p className="mt-1 text-xs text-slate-400">Which NHL title are you modding?</p>
            </header>
            <div className="grid grid-cols-3 gap-3 sm:grid-cols-4">
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
                </button>
              ))}
            </div>
          </>
        )}

        {!autoOpening && step === "platform" && game && (
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
                      <img
                        src={icon}
                        alt={CONSOLE_LABEL[p]}
                        draggable={false}
                        className="h-28 w-auto max-w-full select-none object-contain transition-transform duration-200 ease-out group-hover:scale-105"
                      />
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

        {!autoOpening && step === "root" && game && platform && (
          <>
            <header className="mb-4 flex items-center justify-between gap-2">
              <h1 className="text-xl font-semibold text-slate-100">Point to your game folder</h1>
              {gameTag}
            </header>
            <p className="mb-4 text-xs text-slate-400">{rootHint(platform, game.label)}</p>

            <div className="space-y-3">
              <div>
                <label className="mb-1 block text-xs font-medium text-slate-300">{rootLabel(platform)}</label>
                <div className="flex gap-2">
                  <input
                    className={inputCls + " flex-1 font-mono text-xs"}
                    value={root}
                    onChange={(e) => {
                      setRoot(e.target.value);
                      clearScan();
                    }}
                    placeholder={rootPlaceholder(platform)}
                  />
                  <Button variant="secondary" onClick={() => void browseRoot()}>
                    Browse…
                  </Button>
                  <Button onClick={() => detect.mutate()} disabled={!root.trim() || detect.isPending}>
                    {detect.isPending ? "Scanning…" : "Detect"}
                  </Button>
                </div>
              </div>

              {needsSavePath(platform) && (
                <div>
                  <label className="mb-1 block text-xs font-medium text-slate-300">Save file location</label>
                  <div className="flex gap-2">
                    <input
                      className={inputCls + " flex-1 font-mono text-xs"}
                      value={savePath}
                      onChange={(e) => {
                        setSavePath(e.target.value);
                        clearScan();
                      }}
                      placeholder={savePlaceholder(platform)}
                    />
                    <Button variant="secondary" onClick={() => void browseSave()}>
                      Browse…
                    </Button>
                  </div>
                  <p className="mt-1 text-[11px] text-slate-500">{savePathHint(platform)}</p>
                </div>
              )}
            </div>

            {detect.isError && (
              <div className="mt-4">
                <ErrorBanner title="Couldn't scan that folder" message={String(detect.error)} />
              </div>
            )}

            {report && <InstallSummary report={report} />}

            {report?.extract_needed && (
              <div className="mt-4 rounded-md border border-amber-500/30 bg-amber-500/10 p-3">
                <div className="text-xs font-semibold text-amber-300">Big files need extracting</div>
                <p className="mt-1 text-xs text-amber-300/90">
                  {game.label}'s database is still packed inside its <code>.big</code> archives. Extract them
                  in place, then we'll re-scan.
                </p>
                <div className="mt-2">
                  <Button
                    variant="secondary"
                    onClick={() => extract.mutate()}
                    disabled={extract.isPending || detect.isPending}
                  >
                    {extract.isPending ? "Extracting…" : "Extract big files"}
                  </Button>
                </div>
                {extract.isError && (
                  <p className="mt-2 text-xs text-rose-300">{String(extract.error)}</p>
                )}
              </div>
            )}

            {extractResult && (
              <p className="mt-2 text-[11px] text-slate-400">
                Extracted {extractResult.files_written.toLocaleString()} file
                {extractResult.files_written === 1 ? "" : "s"} from {extractResult.archives} archive
                {extractResult.archives === 1 ? "" : "s"}
                {extractResult.skipped_unsupported > 0 &&
                  ` · ${extractResult.skipped_unsupported} unsupported (chunklzx) skipped`}
                {extractResult.archives_failed > 0 && ` · ${extractResult.archives_failed} archive(s) failed`}
                .
              </p>
            )}

            <div className="mt-5 flex items-center justify-between gap-3">
              <label className="flex cursor-pointer items-center gap-2 text-xs text-slate-300">
                <input
                  type="checkbox"
                  checked={isDefault}
                  onChange={(e) => toggleDefault(e.target.checked)}
                  className="h-3.5 w-3.5 rounded border-slate-600 bg-slate-950 text-sky-600 focus:ring-0"
                />
                Set as Studio default
                <span className="text-slate-500">— load this game &amp; these paths on startup</span>
              </label>
              <Button
                onClick={() => openWorkspace.mutate()}
                disabled={!report || !report.minimum_met || openWorkspace.isPending}
              >
                {openWorkspace.isPending ? "Opening…" : "Open project"}
              </Button>
            </div>

            {openWorkspace.isError && (
              <div className="mt-4">
                <ErrorBanner title="Couldn't open" message={String(openWorkspace.error)} />
              </div>
            )}
          </>
        )}

        <p className="mt-5 border-t border-slate-800 pt-3 text-center text-[11px] text-amber-300/70">
          Beta — built and verified against NHL Legacy. Other titles are in progress.
        </p>
      </div>
    </div>
  );
}

/** What discovery resolved from the chosen root: databases, save inventory, and the modding-minimum gate. */
function InstallSummary({ report }: { report: InstallReportDto }) {
  return (
    <div className="mt-4 rounded-md border border-slate-800 bg-slate-950/40 p-3">
      <div className="mb-2 flex items-center gap-2">
        <span className="text-xs font-semibold text-slate-200">Detected</span>
        {report.has_existing_profile && <Pill tone="info">Existing project — settings loaded</Pill>}
      </div>

      <ul className="space-y-1 text-xs">
        <FileRow label={report.platform === "ps4" ? "nhl_game.db" : "nhlng.db"} path={report.nhlng_path} />
        <FileRow label="metadata" path={report.meta_xml_path} />
        <FileRow label="localization" path={report.localization_path} optional />
      </ul>

      <div className="mt-3">
        <div className="mb-1 text-xs font-semibold text-slate-200">
          Saves ({report.saves.length})
        </div>
        {report.saves.length === 0 ? (
          <p className="text-xs text-slate-500">No save files found.</p>
        ) : (
          <div className="flex flex-wrap gap-1.5">
            {report.saves.map((s, i) => (
              <span
                key={i}
                title={`${s.path} · ${s.container_version}${s.writable ? "" : " · read-only (unsupported version)"}`}
                className={
                  "rounded-full border px-2 py-0.5 text-[11px] " +
                  (s.writable
                    ? "border-emerald-600/40 bg-emerald-500/10 text-emerald-300"
                    : "border-slate-600 bg-slate-800/60 text-slate-400")
                }
              >
                {s.label}
                {!s.writable && " (read-only)"}
              </span>
            ))}
          </div>
        )}
      </div>

      {/* Only nag about the modding minimum once extraction is out of the picture — the extract panel
          explains an un-extracted install on its own. */}
      {!report.minimum_met && !report.extract_needed && (
        <p className="mt-3 rounded-md border border-rose-500/30 bg-rose-500/10 px-3 py-2 text-xs text-rose-300">
          To mod, this folder must have both an <code>nhlng.db</code> and a roster save. One or both are
          missing — check that the game is installed and that the save location points at a roster save.
        </p>
      )}
    </div>
  );
}

// Platform-specific copy for the root-folder step.
function rootBrowseTitle(p: Platform | null): string {
  if (p === "ps3") return "Pick your RPCS3 folder (contains rpcs3.exe)";
  if (p === "ps4") return "Pick your shadPS4 folder (contains shadPS4.exe)";
  if (p === "recomp") return "Pick the recompiled game folder";
  return "Pick the game's install folder (contains default.xex)";
}
function rootLabel(p: Platform | null): string {
  if (p === "ps3") return "RPCS3 folder";
  if (p === "ps4") return "shadPS4 folder";
  if (p === "recomp") return "Game folder";
  return "Game install folder";
}
function rootPlaceholder(p: Platform | null): string {
  if (p === "ps3") return "e.g. C:/RPCS3";
  if (p === "ps4") return "e.g. F:/Emulators/Shadps4";
  if (p === "recomp") return "e.g. C:/Games/NHL Legacy Recompiled";
  return "e.g. C:/Games/NHL14";
}
function rootHint(p: Platform, gameLabel: string): string {
  if (p === "ps3")
    return `Pick your RPCS3 folder (the one containing rpcs3.exe). We'll find ${gameLabel}'s database and save files automatically.`;
  if (p === "ps4")
    return `Pick your shadPS4 folder (the one containing shadPS4.exe). We'll find ${gameLabel}'s nhl_game.db and saves automatically (big files must be extracted first).`;
  if (p === "recomp")
    return `Pick the recompiled game folder, plus where its save lives. If the database is still packed, we'll extract it first.`;
  return `Pick the game's install folder (the one containing default.xex), plus where your Xenia save lives. If the database is still packed, we'll extract it first.`;
}
function savePlaceholder(p: Platform | null): string {
  if (p === "recomp") return "e.g. C:/Games/NHL Legacy Recompiled/saves";
  return "e.g. C:/Users/you/Documents/Xenia/content/…/00000001";
}
function savePathHint(p: Platform | null): string {
  if (p === "recomp") return "The folder holding your roster/profile save.";
  return "Xenia doesn't save to a fixed place — point us at the folder holding your save (or its parent).";
}

function FileRow({ label, path, optional }: { label: string; path: string | null; optional?: boolean }) {
  const ok = !!path;
  return (
    <li className="flex items-center gap-2">
      <span className={ok ? "text-emerald-400" : optional ? "text-slate-500" : "text-rose-400"}>
        {ok ? "✓" : optional ? "—" : "✗"}
      </span>
      <span className="text-slate-300">{label}</span>
      {path && <span className="truncate font-mono text-[11px] text-slate-500">{path}</span>}
    </li>
  );
}
