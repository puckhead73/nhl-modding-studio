import { useState } from "react";
import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";
import { useMutation } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useProject, useAssetRootArgs } from "../lib/project";
import type { PlayerManifestSummaryDto } from "../bindings/PlayerManifestSummaryDto";
import type { Platform } from "../lib/client";
import {
  CONSOLE_LABEL,
  GAMES,
  PLATFORM_ORDER,
  gameById,
  gamesForPlatform,
  img,
} from "../lib/catalog";
import { useSettings, type PathEntry } from "../lib/settings";
import { Button, Card } from "../components/primitives";
import { ExtractionPanel } from "../components/ExtractionPanel";
import { ProjectsStorageField } from "../components/ProjectLocationFields";
import { useQueryClient } from "@tanstack/react-query";

const inputCls =
  "w-full rounded-md border border-slate-700 bg-slate-950 px-2 py-1.5 text-sm text-slate-100 placeholder-slate-600 focus:border-sky-600 focus:outline-none";
const selectCls =
  "w-full rounded-md border border-slate-700 bg-slate-950 px-2 py-1.5 text-sm text-slate-100 focus:border-sky-600 focus:outline-none";

/** Shared frame for the routed Settings subpages (the subtab strip is the app-shell subtab bar). */
function SettingsFrame({ children }: { children: React.ReactNode }) {
  return (
    <div className="mx-auto max-w-3xl">
      <header className="mb-4">
        <h1 className="text-xl font-semibold text-slate-100">Settings</h1>
        <p className="mt-1 text-xs text-slate-400">
          Studio preferences — stored on this machine, independent of any open project.
        </p>
      </header>
      {children}
    </div>
  );
}

export function SettingsGeneralPage() {
  return (
    <SettingsFrame>
      <GeneralTab />
      <AutosaveSettings />
      <ToolsCard />
      <PlayerManifestCard />
    </SettingsFrame>
  );
}

function AutosaveSettings() {
  const { settings, setAutosaveMinutes } = useSettings();
  const [draft, setDraft] = useState(String(settings.autosaveMinutes));
  const minutes = Number(draft);
  const valid = Number.isInteger(minutes) && minutes >= 1 && minutes <= 1440;
  return <Card title="Session autosave" className="mt-4">
    <p className="mb-3 text-xs text-slate-400">Automatically saves changed project files. Default: every 10 minutes.</p>
    <form className="flex items-end gap-3" onSubmit={e => { e.preventDefault(); if (valid) setAutosaveMinutes(minutes); }}>
      <label className="text-sm">Save every (minutes)
        <input type="number" min={1} max={1440} step={1} value={draft} onChange={e => setDraft(e.target.value)} className={inputCls + " mt-1"} />
      </label>
      <Button type="submit" disabled={!valid || minutes === settings.autosaveMinutes}>Apply</Button>
    </form>
    {!valid && <p className="mt-2 text-xs text-amber-300">Enter a whole number from 1 to 1440 minutes.</p>}
    <p role="status" className="mt-2 text-xs text-slate-400">Autosave is set to {settings.autosaveMinutes} minutes.</p>
  </Card>;
}

/// ONB-002 Tools: extract the current project's confirmed game installation through the same
/// preflight, service, progress, cancellation and result as onboarding. Available after Skip; a
/// second extraction against a running destination is refused by the service.
function ToolsCard() {
  const { project, install, info, updateInfo } = useProject();
  const qc = useQueryClient();
  const root =
    info?.destinations.game_dir ?? info?.destinations.install_dir ?? install?.game_dir ?? install?.install_dir ?? info?.destinations.picked_root ?? null;
  const platform = (install?.platform ?? project?.platform ?? null) as Platform | null;
  return (
    <Card title="Tools" subtitle="Extract game assets from your source folder for editing." className="mt-4">
      {!project || !root || !platform ? (
        <p className="text-xs text-slate-400">Choose a game source folder in Project Overview to extract its assets.</p>
      ) : (
        <ExtractionPanel
          initialDestination={info?.destinations.assets_dir ?? undefined}
          projectId={project.project_id}
          onRelink={linked => updateInfo(linked.project, linked.discovery, project.project_id)}
          root={root}
          platform={platform}
          mode="settings"
          onDone={async (job) => {
            if (job && job.state === "done" && info) {
              try {
                updateInfo(await call("project_set_extraction", { projectId: project.project_id, choice: "extracted" }));
              } catch {
                // bookkeeping only
              }
            }
            // Refresh what depends on extracted content (asset roots, listings) without touching edits.
            qc.invalidateQueries();
          }}
        />
      )}
    </Card>
  );
}

/// Player manifest export — one CSV row per player installed in the loaded database: identity,
/// export slug, team, league, and whether a portrait / head mesh actually resolves on disk.
///
/// The art columns are file truth rather than the bio's `lOnr`/`pPaD` id fields, because a player
/// can point at art that was never installed — finding exactly those is the point. That also means
/// the export needs the project's install tree, so it's disabled without one rather than reporting
/// every player as art-less.
function PlayerManifestCard() {
  const { project } = useProject();
  const { installDir, rootArgs } = useAssetRootArgs();
  const [result, setResult] = useState<PlayerManifestSummaryDto | null>(null);
  const [error, setError] = useState<string | null>(null);

  const run = useMutation({
    mutationFn: async () => {
      const outPath = await saveDialog({
        title: "Export player manifest",
        defaultPath: "player-manifest.csv",
        filters: [{ name: "CSV", extensions: ["csv"] }],
      });
      if (!outPath) return null;
      return call("export_player_manifest", {
        projectId: project!.project_id,
        roots: rootArgs,
        outPath,
      });
    },
    onSuccess: (r) => {
      setError(null);
      if (r) setResult(r);
    },
    onError: (e) => {
      setResult(null);
      setError(String(e));
    },
  });

  return (
    <Card
      title="Player manifest (CSV)"
      subtitle="Every player installed in the loaded database — slug, team, league, and which of them are still missing art."
      className="mt-4"
    >
      <div className="space-y-3">
        <p className="text-xs text-slate-400">
          One row per player: player id, bio record, name, export slug, position, team, league, and
          whether a portrait and a head mesh resolve on disk. Portrait and head columns report the
          real installed files — EA's shared placeholder portrait counts as{" "}
          <em className="not-italic text-slate-300">no portrait</em>, and is flagged in its own
          column, so the export is usable as a to-do list.
        </p>
        <div className="flex flex-wrap items-center gap-2">
          <Button onClick={() => run.mutate()} disabled={!installDir || run.isPending}>
            {run.isPending ? "Exporting…" : "Export CSV…"}
          </Button>
          {!installDir && (
            <span className="text-xs text-amber-400">
              This project has no install tree attached, so portrait and head-mesh presence can't be
              checked. Set the install path on the Paths tab first.
            </span>
          )}
        </div>
        {error && <p className="text-xs text-rose-400">{error}</p>}
        {result && (
          <div className="rounded-md border border-slate-800 bg-slate-900/40 px-3 py-2 text-xs text-slate-300">
            <div className="font-medium text-slate-100">
              {result.players.toLocaleString()} players exported
            </div>
            <div className="mt-1 text-slate-400">
              {result.with_portrait.toLocaleString()} with a portrait ·{" "}
              {result.with_head_mesh.toLocaleString()} with a head mesh ·{" "}
              {result.with_slug.toLocaleString()} with a usable slug
            </div>
            <div className="mt-1 truncate font-mono text-[11px] text-slate-500" title={result.path}>
              {result.path}
            </div>
          </div>
        )}
      </div>
    </Card>
  );
}


export function SettingsPathsPage() {
  return (
    <SettingsFrame>
      <Card title="Project storage" className="mb-4"><ProjectsStorageField /></Card>
      <PathsTab />
    </SettingsFrame>
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
        Save the game source folder for each title. Studio reads source files here; Mod Manager chooses where builds are installed.
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
    // A new location invalidates a remembered PS4 roster choice.
    if (typeof picked === "string") patch({ [field]: picked, saveFile: undefined });
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
          <span className="mb-1 block text-[11px] font-medium text-slate-400">
            Game source folder
          </span>
          <div className="flex gap-2">
            <input
              className={inputCls + " flex-1 font-mono text-xs"}
              value={entry.root}
              onChange={(e) => patch({ root: e.target.value, saveFile: undefined })}
              placeholder={platform === "ps4" ? "The game's own folder (or a folder holding your PS4 games)" : "Game / emulator folder"}
            />
            <Button
              variant="secondary"
              onClick={() => void browse("root", `Pick the game source folder for ${label}`)}
            >
              Browse…
            </Button>
          </div>
        </div>

      </div>
    </div>
  );
}
