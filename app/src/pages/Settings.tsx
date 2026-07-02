import { useState } from "react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import type { Platform } from "../lib/client";
import {
  CONSOLE_LABEL,
  GAMES,
  PLATFORM_ORDER,
  gameById,
  gamesForPlatform,
  img,
  needsSavePath,
} from "../lib/catalog";
import { useSettings, type PathEntry } from "../lib/settings";
import { Button, Card } from "../components/primitives";

const inputCls =
  "w-full rounded-md border border-slate-700 bg-slate-950 px-2 py-1.5 text-sm text-slate-100 placeholder-slate-600 focus:border-sky-600 focus:outline-none";
const selectCls =
  "w-full rounded-md border border-slate-700 bg-slate-950 px-2 py-1.5 text-sm text-slate-100 focus:border-sky-600 focus:outline-none";

type SubTab = "general" | "paths";

export function SettingsPage() {
  const [tab, setTab] = useState<SubTab>("general");

  return (
    <div className="mx-auto max-w-3xl">
      <header className="mb-4">
        <h1 className="text-xl font-semibold text-slate-100">Settings</h1>
        <p className="mt-1 text-xs text-slate-400">
          Studio preferences — stored on this machine, independent of any open project.
        </p>
      </header>

      <div className="mb-5 flex gap-1 border-b border-slate-800">
        <SubTabButton active={tab === "general"} onClick={() => setTab("general")}>
          General
        </SubTabButton>
        <SubTabButton active={tab === "paths"} onClick={() => setTab("paths")}>
          Paths
        </SubTabButton>
      </div>

      {tab === "general" ? <GeneralTab /> : <PathsTab />}
    </div>
  );
}

function SubTabButton({
  active,
  onClick,
  children,
}: {
  active: boolean;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={
        "-mb-px border-b-2 px-4 py-2 text-sm font-medium transition-colors " +
        (active
          ? "border-sky-500 text-sky-100"
          : "border-transparent text-slate-400 hover:text-slate-200")
      }
    >
      {children}
    </button>
  );
}

/** General: choose the game + platform the Studio loads into on startup. */
function GeneralTab() {
  const { settings, setDefaultTarget } = useSettings();
  const game = gameById(settings.defaultGameId);
  const platform = settings.defaultPlatform;

  // Changing the game re-homes the platform onto one the new game actually supports.
  function pickGame(gameId: string) {
    if (!gameId) {
      setDefaultTarget(null, null);
      return;
    }
    const g = gameById(gameId);
    if (!g) return;
    const nextPlatform = platform && g.platforms.includes(platform) ? platform : g.platforms[0];
    setDefaultTarget(g.id, nextPlatform);
  }

  return (
    <Card title="Default startup" subtitle="Which title onboarding jumps straight into when you launch the Studio.">
      <div className="grid gap-4 sm:grid-cols-2">
        <label className="block">
          <span className="mb-1 block text-xs font-medium text-slate-300">Default game</span>
          <select className={selectCls} value={game?.id ?? ""} onChange={(e) => pickGame(e.target.value)}>
            <option value="">No default (show game picker)</option>
            {GAMES.map((g) => (
              <option key={g.id} value={g.id}>
                {g.label}
              </option>
            ))}
          </select>
        </label>

        <label className="block">
          <span className="mb-1 block text-xs font-medium text-slate-300">Default console</span>
          <select
            className={selectCls}
            value={platform ?? ""}
            disabled={!game}
            onChange={(e) => game && setDefaultTarget(game.id, (e.target.value || null) as Platform | null)}
          >
            {game ? (
              game.platforms.map((p) => (
                <option key={p} value={p}>
                  {CONSOLE_LABEL[p]}
                </option>
              ))
            ) : (
              <option value="">—</option>
            )}
          </select>
        </label>
      </div>

      <p className="mt-4 text-[11px] text-slate-500">
        {game && platform ? (
          <>
            On startup the Studio opens onboarding at <span className="text-slate-300">{game.label}</span> ·{" "}
            <span className="text-slate-300">{CONSOLE_LABEL[platform]}</span> with its saved paths pre-filled
            (set those under <span className="text-slate-300">Paths</span>).
          </>
        ) : (
          <>No default set — the Studio shows the full game picker on startup.</>
        )}
      </p>
    </Card>
  );
}

/** Paths: a tab per platform, listing that platform's games with editable install + save paths. */
function PathsTab() {
  const [platform, setPlatform] = useState<Platform>("ps3");
  const games = gamesForPlatform(platform);

  return (
    <div>
      <div className="mb-4 flex flex-wrap gap-2">
        {PLATFORM_ORDER.map((p) => {
          const active = p === platform;
          return (
            <button
              key={p}
              type="button"
              onClick={() => setPlatform(p)}
              className={
                "rounded-md border px-3 py-1.5 text-sm font-medium transition-colors " +
                (active
                  ? "border-sky-600 bg-sky-900/40 text-sky-100"
                  : "border-slate-700 text-slate-300 hover:bg-slate-800/60")
              }
            >
              {CONSOLE_LABEL[p]}
            </button>
          );
        })}
      </div>

      <p className="mb-3 text-[11px] text-slate-500">
        {needsSavePath(platform)
          ? "Save the install folder and the save-file folder for each title on this console."
          : "Save the install folder for each title. Saves are found automatically from the emulator root, so there's no save-file field here."}
      </p>

      <div className="space-y-3">
        {games.map((g) => (
          <PathRow key={g.id} gameId={g.id} label={g.label} file={g.file} platform={platform} />
        ))}
      </div>
    </div>
  );
}

function PathRow({
  gameId,
  label,
  file,
  platform,
}: {
  gameId: string;
  label: string;
  file: string;
  platform: Platform;
}) {
  const { getPathEntry, setPathEntry } = useSettings();
  const entry = getPathEntry(gameId, platform);
  const icon = img(file);

  function patch(next: Partial<PathEntry>) {
    setPathEntry(gameId, platform, { ...entry, ...next });
  }

  async function browse(field: keyof PathEntry, title: string) {
    const picked = await openDialog({ multiple: false, directory: true, title });
    if (typeof picked === "string") patch({ [field]: picked });
  }

  return (
    <div className="rounded-lg border border-slate-800 bg-slate-900/40 p-3">
      <div className="mb-2 flex items-center gap-2">
        {icon ? (
          <img src={icon} alt={label} draggable={false} className="h-8 w-auto select-none rounded object-contain" />
        ) : null}
        <span className="text-sm font-semibold text-slate-100">{label}</span>
      </div>

      <div className="space-y-2">
        <div>
          <span className="mb-1 block text-[11px] font-medium text-slate-400">Install folder</span>
          <div className="flex gap-2">
            <input
              className={inputCls + " flex-1 font-mono text-xs"}
              value={entry.root}
              onChange={(e) => patch({ root: e.target.value })}
              placeholder="Game / emulator folder"
            />
            <Button variant="secondary" onClick={() => void browse("root", `Pick the install folder for ${label}`)}>
              Browse…
            </Button>
          </div>
        </div>

        {needsSavePath(platform) && (
          <div>
            <span className="mb-1 block text-[11px] font-medium text-slate-400">Save-file folder</span>
            <div className="flex gap-2">
              <input
                className={inputCls + " flex-1 font-mono text-xs"}
                value={entry.savePath}
                onChange={(e) => patch({ savePath: e.target.value })}
                placeholder="Folder holding the roster/profile save"
              />
              <Button
                variant="secondary"
                onClick={() => void browse("savePath", `Pick the save-file folder for ${label}`)}
              >
                Browse…
              </Button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
