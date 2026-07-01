import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { useMutation } from "@tanstack/react-query";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { call, type Platform, type Role } from "../lib/client";
import type { InstallReportDto } from "../bindings/InstallReportDto";
import { useProject } from "../lib/project";
import { Button, ErrorBanner, Pill } from "../components/primitives";
import headerLogo from "../assets/ui/NHL Modding Studio Header.png";

// Resolve every UI image once (game box-art, console icons) by filename.
const UI_IMAGES = import.meta.glob("../assets/ui/*.png", {
  eager: true,
  query: "?url",
  import: "default",
}) as Record<string, string>;
function img(file: string): string {
  const hit = Object.entries(UI_IMAGES).find(([p]) => p.endsWith("/" + file));
  return hit ? hit[1] : "";
}

type GameDef = {
  id: string;
  label: string;
  file: string;
  platforms: Platform[];
};

const PS3_XBOX: Platform[] = ["ps3", "xbox"];
const PS3_XBOX_PS4: Platform[] = ["ps3", "xbox", "ps4"];

// Every ingested title, in release order. Multi-system titles prompt for the console after selection;
// NHL 15 spans PS3/Xbox/PS4 (the bridge title); NHL 16–21 are PS4; Legacy Recompiled is PC (recomp).
const GAMES: GameDef[] = [
  { id: "nhl10", label: "NHL 10", file: "nhl10.png", platforms: PS3_XBOX },
  { id: "nhl11", label: "NHL 11", file: "nhl11.png", platforms: PS3_XBOX },
  { id: "nhl12", label: "NHL 12", file: "nhl12.png", platforms: PS3_XBOX },
  { id: "nhl13", label: "NHL 13", file: "nhl13.png", platforms: PS3_XBOX },
  { id: "nhl14", label: "NHL 14", file: "nhl14.png", platforms: PS3_XBOX },
  { id: "nhl15", label: "NHL 15", file: "nhl15.png", platforms: PS3_XBOX_PS4 },
  { id: "nhl16", label: "NHL 16", file: "nhl16.png", platforms: ["ps4"] },
  { id: "nhllegacy", label: "NHL Legacy", file: "nhllegacy.png", platforms: PS3_XBOX },
  { id: "legacyrecomp", label: "NHL Legacy Recompiled", file: "legacyrecomp.png", platforms: ["recomp"] },
  { id: "nhl17", label: "NHL 17", file: "nhl17.png", platforms: ["ps4"] },
  { id: "nhl18", label: "NHL 18", file: "nhl18.png", platforms: ["ps4"] },
  { id: "nhl19", label: "NHL 19", file: "nhl19.png", platforms: ["ps4"] },
  { id: "nhl20", label: "NHL 20", file: "nhl20.png", platforms: ["ps4"] },
  { id: "nhl21", label: "NHL 21", file: "nhl21.png", platforms: ["ps4"] },
];

const CONSOLE_LABEL: Record<Platform, string> = {
  ps3: "PS3",
  xbox: "Xbox 360",
  ps4: "PS4",
  recomp: "NHL Legacy Recompiled",
};

// Console icon by platform (PS4/Recomp have no icon yet — the picker falls back to a text card).
const CONSOLE_ICON: Partial<Record<Platform, string>> = {
  ps3: "PS3.png",
  xbox: "360.png",
};

// PS3/Xbox/PS4 use the root-folder onboarding (RPCS3 / install dir / shadPS4); Recomp falls back to
// the single-file pick path.
const ROOT_PLATFORMS: Platform[] = ["ps3", "xbox", "ps4"];

type Step = "game" | "platform" | "root" | "bigfiles" | "file";

const inputCls =
  "w-full rounded-md border border-slate-700 bg-slate-950 px-2 py-1.5 text-sm text-slate-100 placeholder-slate-600 focus:border-sky-600 focus:outline-none";

export function OpenProject() {
  const { open } = useProject();
  const navigate = useNavigate();

  const [step, setStep] = useState<Step>("game");
  const [game, setGame] = useState<GameDef | null>(null);
  const [platform, setPlatform] = useState<Platform | null>(null);

  // Root-folder onboarding (PS3/Xbox).
  const [root, setRoot] = useState("");
  const [xeniaRoot, setXeniaRoot] = useState("");
  const [report, setReport] = useState<InstallReportDto | null>(null);
  const [bigFilesExtracted, setBigFilesExtracted] = useState<boolean | null>(null);

  // Legacy single-file path (PS4/Recomp + Advanced override).
  const [role, setRole] = useState<Role>("nhlng");
  const [dbPath, setDbPath] = useState("");
  const [advanced, setAdvanced] = useState(false);
  const [metaXml, setMetaXml] = useState("");
  const [defsRoot, setDefsRoot] = useState("");

  const detect = useMutation({
    mutationFn: () =>
      call("detect_install", {
        gameId: game!.id,
        platform: platform!,
        root,
        xeniaRoot: xeniaRoot.trim() || null,
      }),
    onSuccess: (r) => setReport(r),
  });

  const openWorkspace = useMutation({
    mutationFn: () =>
      call("open_workspace", {
        gameId: game!.id,
        platform: platform!,
        root,
        xeniaRoot: xeniaRoot.trim() || null,
        bigFilesExtracted: bigFilesExtracted === true,
      }),
    onSuccess: (summary) => {
      open(summary, report?.nhlng_path ?? root, report);
      navigate("/app/dashboard");
    },
  });

  const extract = useMutation({
    mutationFn: () => call("extract_big_files", { gameId: game!.id, platform: platform!, root }),
  });

  const load = useMutation({
    mutationFn: () =>
      advanced
        ? call("load_project", { dbPath, platform: platform!, role, metaXml, defsRoot })
        : call("open_project", { dbPath, platform: platform!, role }),
    onSuccess: (summary) => {
      open(summary, dbPath);
      navigate("/app/dashboard");
    },
  });

  function resetFlow() {
    setReport(null);
    setRoot("");
    setXeniaRoot("");
    setBigFilesExtracted(null);
    setDbPath("");
    setStep("game");
  }

  function pickGame(g: GameDef) {
    setGame(g);
    setReport(null);
    if (g.platforms.length === 1) {
      const p = g.platforms[0];
      setPlatform(p);
      setStep(ROOT_PLATFORMS.includes(p) ? "root" : "file");
    } else {
      setPlatform(null);
      setStep("platform");
    }
  }

  function pickPlatform(p: Platform) {
    setPlatform(p);
    setReport(null);
    setStep(ROOT_PLATFORMS.includes(p) ? "root" : "file");
  }

  async function browseRoot() {
    const picked = await openDialog({
      multiple: false,
      directory: true,
      title: rootBrowseTitle(platform),
    });
    if (typeof picked === "string") {
      setRoot(picked);
      setReport(null);
    }
  }

  async function browseXenia() {
    const picked = await openDialog({ multiple: false, directory: true, title: "Pick the Xenia content folder" });
    if (typeof picked === "string") setXeniaRoot(picked);
  }

  async function browseFile() {
    const picked = await openDialog({
      multiple: false,
      directory: false,
      title: "Pick the nhlng.db (or a save container)",
    });
    if (typeof picked === "string") setDbPath(picked);
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

        {step === "game" && (
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

        {step === "root" && game && platform && (
          <>
            <header className="mb-4 flex items-center justify-between gap-2">
              <h1 className="text-xl font-semibold text-slate-100">Point to your game folder</h1>
              {gameTag}
            </header>
            <p className="mb-4 text-xs text-slate-400">
              {platform === "ps3" ? (
                <>
                  Pick your <code className="text-slate-300">RPCS3</code> folder (the one containing{" "}
                  <code className="text-slate-300">rpcs3.exe</code>). We'll find {game.label}'s database and
                  save files automatically.
                </>
              ) : platform === "ps4" ? (
                <>
                  Pick your <code className="text-slate-300">shadPS4</code> folder (the one containing{" "}
                  <code className="text-slate-300">shadPS4.exe</code>). We'll find {game.label}'s{" "}
                  <code className="text-slate-300">nhl_game.db</code> and saves automatically (the big files must
                  be extracted first).
                </>
              ) : (
                <>
                  Pick the game's install folder (the one containing{" "}
                  <code className="text-slate-300">default.xex</code>). Saves are read from your Xenia content
                  folder.
                </>
              )}
            </p>

            <div className="space-y-3">
              <div>
                <label className="mb-1 block text-xs font-medium text-slate-300">{rootLabel(platform)}</label>
                <div className="flex gap-2">
                  <input
                    className={inputCls + " flex-1 font-mono text-xs"}
                    value={root}
                    onChange={(e) => {
                      setRoot(e.target.value);
                      setReport(null);
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

              {platform === "xbox" && (
                <details className="rounded-md border border-slate-800 bg-slate-950/40 px-3 py-2">
                  <summary className="cursor-pointer text-xs font-medium text-slate-400">
                    Advanced — override Xenia content folder
                  </summary>
                  <div className="mt-2 flex gap-2">
                    <input
                      className={inputCls + " flex-1 font-mono text-xs"}
                      value={xeniaRoot}
                      onChange={(e) => setXeniaRoot(e.target.value)}
                      placeholder="e.g. C:/Users/you/Documents/Xenia/content"
                    />
                    <Button variant="secondary" onClick={() => void browseXenia()}>
                      Browse…
                    </Button>
                  </div>
                </details>
              )}
            </div>

            {detect.isError && (
              <div className="mt-4">
                <ErrorBanner title="Couldn't scan that folder" message={String(detect.error)} />
              </div>
            )}

            {report && <InstallSummary report={report} />}

            <div className="mt-5 flex items-center justify-end gap-3">
              <Button
                onClick={() => setStep("bigfiles")}
                disabled={!report || !report.minimum_met}
              >
                Continue →
              </Button>
            </div>
          </>
        )}

        {step === "bigfiles" && game && platform && (
          <>
            <header className="mb-4 flex items-center justify-between gap-2">
              <h1 className="text-xl font-semibold text-slate-100">Big files</h1>
              {gameTag}
            </header>
            <p className="mb-2 text-sm text-slate-300">Do you need {game.label}'s big files extracted?</p>
            <p className="mb-4 rounded-md border border-amber-500/30 bg-amber-500/10 px-3 py-2 text-xs text-amber-300/90">
              Any modding requires the game's big files to be extracted first.
            </p>

            <div className="grid grid-cols-2 gap-3">
              <button
                type="button"
                onClick={() => setBigFilesExtracted(true)}
                className={
                  "rounded-lg border p-3 text-left text-sm transition-colors " +
                  (bigFilesExtracted === true
                    ? "border-sky-500 bg-sky-500/10 text-slate-100"
                    : "border-slate-700 bg-slate-950 text-slate-300 hover:border-slate-500")
                }
              >
                <div className="font-semibold">Already extracted</div>
                <div className="mt-0.5 text-xs text-slate-400">My big files are unpacked and ready.</div>
              </button>
              <button
                type="button"
                onClick={() => setBigFilesExtracted(false)}
                className={
                  "rounded-lg border p-3 text-left text-sm transition-colors " +
                  (bigFilesExtracted === false
                    ? "border-sky-500 bg-sky-500/10 text-slate-100"
                    : "border-slate-700 bg-slate-950 text-slate-300 hover:border-slate-500")
                }
              >
                <div className="font-semibold">I need them extracted</div>
                <div className="mt-0.5 text-xs text-slate-400">Run extraction before modding.</div>
              </button>
            </div>

            {bigFilesExtracted === false && (
              <div className="mt-3">
                <Button variant="secondary" onClick={() => extract.mutate()} disabled={extract.isPending}>
                  {extract.isPending ? "Extracting…" : "Extract big files"}
                </Button>
                {extract.isError && (
                  <p className="mt-2 text-xs text-amber-300/80">{String(extract.error)}</p>
                )}
              </div>
            )}

            <div className="mt-5 flex items-center justify-between gap-3">
              <button
                type="button"
                onClick={() => setStep("root")}
                className="text-[11px] text-sky-400 underline-offset-2 hover:underline"
              >
                ← Back
              </button>
              <Button
                onClick={() => openWorkspace.mutate()}
                disabled={bigFilesExtracted === null || openWorkspace.isPending}
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

        {step === "file" && game && platform && (
          <>
            <header className="mb-4 flex items-center justify-between gap-2">
              <h1 className="text-xl font-semibold text-slate-100">Open your EA NHL database</h1>
              {gameTag}
            </header>
            <p className="mb-4 text-xs text-slate-400">
              Pick <code className="text-slate-300">{platform === "ps4" ? "nhl_game.db" : "nhlng.db"}</code>
              {platform === "ps4" ? " (PS4's main database)" : " (or a save container)"}. The metadata XML
              beside it and the bundled defs are detected automatically.
            </p>

            <div className="space-y-3">
              <div>
                <label className="mb-1 block text-xs font-medium text-slate-300">Database / save</label>
                <div className="flex gap-2">
                  <input
                    className={inputCls + " flex-1 font-mono text-xs"}
                    value={dbPath}
                    onChange={(e) => setDbPath(e.target.value)}
                    placeholder={
                      platform === "ps4"
                        ? "e.g. C:/NHL21/databases/nhl_game.db"
                        : "e.g. C:/NHL/extracted/db/nhlng.db"
                    }
                  />
                  <Button variant="secondary" onClick={() => void browseFile()}>
                    Browse…
                  </Button>
                </div>
              </div>
              <div className="w-40">
                <label className="mb-1 block text-xs font-medium text-slate-300">Role</label>
                <select className={inputCls} value={role} onChange={(e) => setRole(e.target.value as Role)}>
                  <option value="nhlng">{platform === "ps4" ? "nhl_game.db" : "nhlng.db"}</option>
                  <option value="roster">Roster save</option>
                  <option value="localization">Localization</option>
                </select>
              </div>

              <details className="rounded-md border border-slate-800 bg-slate-950/40 px-3 py-2" open={advanced}>
                <summary
                  className="cursor-pointer text-xs font-medium text-slate-400"
                  onClick={(e) => {
                    e.preventDefault();
                    setAdvanced((a) => !a);
                  }}
                >
                  Advanced — override metadata / defs paths
                </summary>
                {advanced && (
                  <div className="mt-2 space-y-2">
                    <input
                      className={inputCls + " font-mono text-xs"}
                      value={metaXml}
                      onChange={(e) => setMetaXml(e.target.value)}
                      placeholder="metadata XML path (nhlng-meta.xml)"
                    />
                    <input
                      className={inputCls + " font-mono text-xs"}
                      value={defsRoot}
                      onChange={(e) => setDefsRoot(e.target.value)}
                      placeholder="defs root (contains enums/ and relationships/)"
                    />
                  </div>
                )}
              </details>
            </div>

            <div className="mt-5 flex items-center justify-end gap-3">
              <Button
                onClick={() => load.mutate()}
                disabled={load.isPending || !dbPath.trim() || (advanced && (!metaXml.trim() || !defsRoot.trim()))}
              >
                {load.isPending ? "Loading…" : "Load"}
              </Button>
            </div>

            {load.isError && (
              <div className="mt-4">
                <ErrorBanner title="Couldn't load" message={String(load.error)} />
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

      {!report.minimum_met && (
        <p className="mt-3 rounded-md border border-rose-500/30 bg-rose-500/10 px-3 py-2 text-xs text-rose-300">
          To mod, this folder must have both an <code>nhlng.db</code> and a roster save. One or both are
          missing — check that the game is installed and you've created a roster save in-game.
        </p>
      )}
    </div>
  );
}

// Platform-specific copy for the root-folder step.
function rootBrowseTitle(p: Platform | null): string {
  if (p === "ps3") return "Pick your RPCS3 folder (contains rpcs3.exe)";
  if (p === "ps4") return "Pick your shadPS4 folder (contains shadPS4.exe)";
  return "Pick the game's install folder (contains default.xex)";
}
function rootLabel(p: Platform | null): string {
  if (p === "ps3") return "RPCS3 folder";
  if (p === "ps4") return "shadPS4 folder";
  return "Game install folder";
}
function rootPlaceholder(p: Platform | null): string {
  if (p === "ps3") return "e.g. C:/RPCS3";
  if (p === "ps4") return "e.g. F:/Emulators/Shadps4";
  return "e.g. C:/Games/NHL14";
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
