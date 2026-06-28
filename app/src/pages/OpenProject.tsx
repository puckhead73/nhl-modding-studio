import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { useMutation } from "@tanstack/react-query";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { call, type Platform, type Role } from "../lib/client";
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
  recommended?: boolean;
};

const PS3_XBOX: Platform[] = ["ps3", "xbox"];

// Every ingested title, in release order. Multi-system titles prompt for PS3/Xbox after selection;
// Legacy Recompiled is PC (recomp); NHL 17–21 are PS4.
const GAMES: GameDef[] = [
  { id: "nhl10", label: "NHL 10", file: "nhl10.png", platforms: PS3_XBOX },
  { id: "nhl11", label: "NHL 11", file: "nhl11.png", platforms: PS3_XBOX },
  { id: "nhl12", label: "NHL 12", file: "nhl12.png", platforms: PS3_XBOX },
  { id: "nhl13", label: "NHL 13", file: "nhl13.png", platforms: PS3_XBOX },
  { id: "nhl14", label: "NHL 14", file: "nhl14.png", platforms: PS3_XBOX },
  { id: "nhl15", label: "NHL 15", file: "nhl15.png", platforms: PS3_XBOX },
  { id: "nhllegacy", label: "NHL Legacy", file: "nhllegacy.png", platforms: PS3_XBOX },
  {
    id: "legacyrecomp",
    label: "NHL Legacy Recompiled",
    file: "legacyrecomp.png",
    platforms: ["recomp"],
    recommended: true,
  },
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

type Step = "game" | "platform" | "file";

export function OpenProject() {
  const { open } = useProject();
  const navigate = useNavigate();

  const [step, setStep] = useState<Step>("game");
  const [game, setGame] = useState<GameDef | null>(null);
  const [platform, setPlatform] = useState<Platform | null>(null);
  const [role, setRole] = useState<Role>("nhlng");
  const [dbPath, setDbPath] = useState("");
  const [advanced, setAdvanced] = useState(false);
  const [metaXml, setMetaXml] = useState("");
  const [defsRoot, setDefsRoot] = useState("");

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

  function pickGame(g: GameDef) {
    setGame(g);
    if (g.platforms.length === 1) {
      setPlatform(g.platforms[0]);
      setStep("file");
    } else {
      setPlatform(null);
      setStep("platform");
    }
  }

  async function browse() {
    const picked = await openDialog({
      multiple: false,
      directory: false,
      title: "Pick the nhlng.db (or a save container)",
    });
    if (typeof picked === "string") setDbPath(picked);
  }

  const input =
    "w-full rounded-md border border-slate-700 bg-slate-950 px-2 py-1.5 text-sm text-slate-100 placeholder-slate-600 focus:border-sky-600 focus:outline-none";

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
                  <img
                    src={img(g.file)}
                    alt={g.label}
                    draggable={false}
                    className="h-24 w-auto max-w-full select-none object-contain transition-transform duration-200 ease-out group-hover:scale-105"
                  />
                  <span className="text-center text-xs font-semibold text-slate-100">{g.label}</span>
                  {g.recommended && (
                    <span className="rounded-full border border-emerald-600/70 bg-emerald-950/50 px-2 py-0.5 text-[9px] font-semibold uppercase tracking-wide text-emerald-300">
                      Recommended
                    </span>
                  )}
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
                <button
                  type="button"
                  className="underline hover:text-slate-200"
                  onClick={() => setStep("game")}
                >
                  Change game
                </button>
              </p>
            </header>
            <div className="mx-auto grid max-w-md grid-cols-2 gap-4">
              {game.platforms.map((p) => (
                <button
                  key={p}
                  type="button"
                  onClick={() => {
                    setPlatform(p);
                    setStep("file");
                  }}
                  className="group flex cursor-pointer flex-col items-center gap-2 rounded-lg border border-slate-700 bg-slate-950 p-4 transition-colors hover:border-sky-500"
                >
                  <img
                    src={img(p === "ps3" ? "PS3.png" : "360.png")}
                    alt={CONSOLE_LABEL[p]}
                    draggable={false}
                    className="h-28 w-auto max-w-full select-none object-contain transition-transform duration-200 ease-out group-hover:scale-105"
                  />
                  <span className="text-sm font-semibold text-slate-100">{CONSOLE_LABEL[p]}</span>
                </button>
              ))}
            </div>
          </>
        )}

        {step === "file" && game && platform && (
          <>
            <header className="mb-4 flex items-center justify-between gap-2">
              <h1 className="text-xl font-semibold text-slate-100">Open your EA NHL database</h1>
              <div className="flex items-center gap-2">
                <Pill tone="info">
                  {game.label} · {CONSOLE_LABEL[platform]}
                </Pill>
                <button
                  type="button"
                  onClick={() => setStep("game")}
                  className="text-[11px] text-sky-400 underline-offset-2 hover:underline"
                >
                  ← Change game
                </button>
              </div>
            </header>
            <p className="mb-4 text-xs text-slate-400">
              Pick <code className="text-slate-300">nhlng.db</code> (or a save container). The
              metadata XML beside it and the bundled defs are detected automatically.
            </p>

            <div className="space-y-3">
              <div>
                <label className="mb-1 block text-xs font-medium text-slate-300">Database / save</label>
                <div className="flex gap-2">
                  <input
                    className={input + " flex-1 font-mono text-xs"}
                    value={dbPath}
                    onChange={(e) => setDbPath(e.target.value)}
                    placeholder="e.g. C:/NHL/extracted/db/nhlng.db"
                  />
                  <Button variant="secondary" onClick={() => void browse()}>
                    Browse…
                  </Button>
                </div>
              </div>
              <div className="w-40">
                <label className="mb-1 block text-xs font-medium text-slate-300">Role</label>
                <select className={input} value={role} onChange={(e) => setRole(e.target.value as Role)}>
                  <option value="nhlng">nhlng.db</option>
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
                      className={input + " font-mono text-xs"}
                      value={metaXml}
                      onChange={(e) => setMetaXml(e.target.value)}
                      placeholder="metadata XML path (nhlng-meta.xml)"
                    />
                    <input
                      className={input + " font-mono text-xs"}
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
