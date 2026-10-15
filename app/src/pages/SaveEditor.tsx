// Save Editor — the launch screen's standalone tool. Pick one save (PS3 SYS-DATA, an Xbox 360
// save file, PS4 DATA) or an extracted database; the backend identifies its format, kind, game and
// platform against Studio's bundled game definitions, and the file opens in the same database
// editor Mod Studio uses — trimmed to database surfaces, with no project and no game installation.
import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { useMutation } from "@tanstack/react-query";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import headerLogo from "../assets/ui/NHL Modding Studio Header.png";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import { GAMES, gameById, img } from "../lib/catalog";
import { Button, Card, Pill, Spinner } from "../components/primitives";
import type { SaveFileIdentityDto } from "../bindings/SaveFileIdentityDto";
import type { SaveCandidateDto } from "../bindings/SaveCandidateDto";

const PLATFORM_LABEL: Record<string, string> = {
  ps3: "PlayStation 3",
  xbox: "Xbox 360",
  ps4: "PlayStation 4",
  recomp: "Legacy Recompiled",
};

const key = (c: { game_id: string; platform: string }) => `${c.game_id}/${c.platform}`;

export function SaveEditorPage() {
  const navigate = useNavigate();
  const { open } = useProject();
  const [identity, setIdentity] = useState<SaveFileIdentityDto | null>(null);
  const [choice, setChoice] = useState<string | null>(null);
  const [acknowledged, setAcknowledged] = useState(false);
  const [manualMeta, setManualMeta] = useState<string | null>(null);
  const [manualGame, setManualGame] = useState<string>("");
  const [manualPlatform, setManualPlatform] = useState<string>("");

  const identify = useMutation({
    mutationFn: (path: string) => call("save_editor_identify", { path }),
    onSuccess: (id) => {
      setIdentity(id);
      const first = id.candidates.find((c) => c.leading) ?? id.candidates[0];
      setChoice(first ? key(first) : null);
      setAcknowledged(false);
      setManualMeta(null);
      setManualGame("");
      setManualPlatform("");
    },
    onError: () => setIdentity(null),
  });

  const manual = identity != null && identity.candidates.length === 0 && !identity.change_list
    && !["settings", "hockeycard"].includes(identity.kind);
  const chosen: { game_id: string; platform: string } | null = manual
    ? manualMeta && manualGame && manualPlatform ? { game_id: manualGame, platform: manualPlatform } : null
    : identity?.candidates.find((c) => key(c) === choice) ?? null;

  const openFile = useMutation({
    mutationFn: async () => {
      if (!identity || !chosen) throw new Error("Choose the game first.");
      return call("save_editor_open", {
        path: identity.path,
        gameId: chosen.game_id,
        platform: chosen.platform,
        acknowledged,
        metaOverride: manual ? manualMeta : null,
      });
    },
    onSuccess: (dto) => {
      open(dto.summary, dto.file.path, null, null, null, dto);
      navigate("/app/database/teams");
    },
  });

  async function pickFile() {
    const picked = await openDialog({
      multiple: false,
      directory: false,
      title: "Choose a save file or an extracted game database",
    });
    if (typeof picked === "string") identify.mutate(picked);
  }

  async function pickMeta() {
    const picked = await openDialog({
      multiple: false,
      directory: false,
      title: "Choose the game's metadata file",
      filters: [{ name: "Metadata", extensions: ["xml"] }],
    });
    if (typeof picked === "string") setManualMeta(picked);
  }

  const refused = identity?.decision === "refused" && !manual;
  const needsAck = identity?.decision === "caution";
  const canOpen = !!identity && !!chosen && !refused && (!needsAck || acknowledged) && !openFile.isPending;

  return (
    <main className="min-h-screen bg-slate-950 p-6 text-slate-100">
      <div className="mx-auto max-w-3xl space-y-5">
        <div className="flex items-center justify-between gap-4">
          <div className="flex items-center gap-4">
            <img src={headerLogo} alt="NHL Modding Studio" className="h-10" draggable={false} />
            <h1 className="text-xl font-semibold">Save Editor</h1>
          </div>
          <Button variant="secondary" onClick={() => navigate("/")}>Back</Button>
        </div>

        <Card
          title="Open a save or database"
          subtitle="Studio works out which game and platform the file belongs to. Nothing is changed until you save."
        >
          <div className="flex flex-wrap items-center gap-3">
            <Button onClick={() => void pickFile()} disabled={identify.isPending}>
              {identity ? "Choose another file…" : "Choose file…"}
            </Button>
            {identify.isPending && <Spinner label="Identifying the file…" />}
          </div>
          <ul className="mt-3 list-disc space-y-0.5 pl-5 text-xs text-slate-400">
            <li>PS3: the <span className="font-mono">SYS-DATA</span> file inside the save's folder (for example <span className="font-mono">BLUS31179…-ROSTER</span>).</li>
            <li>Xbox 360: the save file itself (for example <span className="font-mono">ROSTER</span>).</li>
            <li>PS4: the <span className="font-mono">DATA</span> file inside the save's folder.</li>
            <li>Or an extracted game database such as <span className="font-mono">nhlng.db</span> or <span className="font-mono">nhl_game.db</span>.</li>
          </ul>
          {identify.isError && (
            <p role="alert" className="mt-3 rounded-md border border-rose-800 bg-rose-950/60 px-3 py-2 text-sm text-rose-100">
              {String(identify.error)}
            </p>
          )}
        </Card>

        {identity && (
          <IdentityCard
            identity={identity}
            choice={choice}
            onChoose={setChoice}
          />
        )}

        {identity && manual && (
          <Card
            title="Advanced: choose the game yourself"
            subtitle="Use the metadata file that came with the same game and update. Studio still checks that it describes every table and field in this file."
          >
            <div className="grid gap-3 sm:grid-cols-3">
              <label className="text-sm">
                <span className="mb-1 block text-xs text-slate-400">Game</span>
                <select
                  className="w-full rounded-md border border-slate-700 bg-slate-900 px-2 py-1.5"
                  value={manualGame}
                  onChange={(e) => { setManualGame(e.target.value); setManualPlatform(""); }}
                >
                  <option value="">Choose…</option>
                  {GAMES.map((g) => <option key={g.id} value={g.id}>{g.label}</option>)}
                </select>
              </label>
              <label className="text-sm">
                <span className="mb-1 block text-xs text-slate-400">Platform</span>
                <select
                  className="w-full rounded-md border border-slate-700 bg-slate-900 px-2 py-1.5"
                  value={manualPlatform}
                  disabled={!manualGame}
                  onChange={(e) => setManualPlatform(e.target.value)}
                >
                  <option value="">Choose…</option>
                  {(gameById(manualGame)?.platforms ?? []).map((p) => <option key={p} value={p}>{PLATFORM_LABEL[p]}</option>)}
                </select>
              </label>
              <div className="text-sm">
                <span className="mb-1 block text-xs text-slate-400">Metadata file</span>
                <Button variant="secondary" onClick={() => void pickMeta()}>{manualMeta ? "Change…" : "Choose…"}</Button>
              </div>
            </div>
            {manualMeta && <p className="mt-2 break-all font-mono text-xs text-slate-400">{manualMeta}</p>}
          </Card>
        )}

        {identity && !refused && (
          <div className="space-y-3">
            {needsAck && (
              <label className="flex items-start gap-2 rounded-md border border-amber-800/70 bg-amber-950/40 p-3 text-sm text-amber-100">
                <input type="checkbox" className="mt-1" checked={acknowledged} onChange={(e) => setAcknowledged(e.target.checked)} />
                <span>I understand. Open this save anyway.</span>
              </label>
            )}
            <div className="flex items-center justify-end gap-3">
              {openFile.isError && <p role="alert" className="text-sm text-rose-300">{String(openFile.error)}</p>}
              <Button onClick={() => openFile.mutate()} disabled={!canOpen}>
                {openFile.isPending ? "Opening…" : "Open in Save Editor"}
              </Button>
            </div>
          </div>
        )}
      </div>
    </main>
  );
}

function IdentityCard({
  identity,
  choice,
  onChoose,
}: {
  identity: SaveFileIdentityDto;
  choice: string | null;
  onChoose: (k: string) => void;
}) {
  const leading = identity.candidates.filter((c) => c.leading);
  const others = identity.candidates.filter((c) => !c.leading);
  const selected = identity.candidates.find((c) => key(c) === choice) ?? leading[0] ?? null;
  const game = gameById(selected?.game_id);
  const art = game ? img(game.file) : "";
  // Several leading candidates: either the platform (same game) or the game itself is open.
  const choose = leading.length > 1;
  return (
    <Card title={identity.file_name} subtitle={identity.format_label}>
      <div className="flex gap-4">
        {art && <img src={art} alt="" className="h-28 w-auto shrink-0 rounded-md border border-slate-800 object-contain" draggable={false} />}
        <div className="min-w-0 flex-1 space-y-2">
          <div className="flex flex-wrap items-center gap-2">
            <Pill tone="info">{identity.kind_label}</Pill>
            {selected && <span className="text-lg font-semibold">{selected.title}</span>}
            {selected && <span className="text-sm text-slate-400">{selected.platform_label}</span>}
          </div>
          {identity.candidates.length === 0 && identity.decision !== "refused" && (
            <p className="text-sm text-slate-300">Studio could not match this file to one of its bundled games.</p>
          )}
          {choose && (
            <fieldset className="space-y-1">
              <legend className="mb-1 text-xs text-slate-400">
                {identity.game_confident
                  ? "This file's layout is the same on several platforms. Choose the one it came from:"
                  : "This file matches more than one game. Choose the one it came from:"}
              </legend>
              {leading.map((c) => (
                <CandidateRadio key={key(c)} c={c} checked={key(c) === choice} onChoose={onChoose} />
              ))}
            </fieldset>
          )}
          {!choose && others.length > 0 && (
            <details className="text-xs text-slate-400">
              <summary className="cursor-pointer">Other games whose definitions also fit ({others.length})</summary>
              <div className="mt-1 space-y-1">
                {[...leading, ...others].map((c) => (
                  <CandidateRadio key={key(c)} c={c} checked={key(c) === choice} onChoose={onChoose} />
                ))}
              </div>
            </details>
          )}
        </div>
      </div>

      {identity.decision_reason && (
        <p
          className={
            "mt-4 rounded-md border px-3 py-2 text-sm " +
            (identity.decision === "refused"
              ? "border-rose-800 bg-rose-950/60 text-rose-100"
              : "border-amber-800/70 bg-amber-950/40 text-amber-100")
          }
        >
          {identity.decision_reason}
        </p>
      )}
      {identity.notes.length > 0 && (
        <ul className="mt-3 list-disc space-y-1 pl-5 text-xs text-slate-300">
          {identity.notes.map((n, i) => <li key={i}>{n}</li>)}
        </ul>
      )}

      <details className="mt-4 text-xs text-slate-400">
        <summary className="cursor-pointer">Details</summary>
        <dl className="mt-2 grid grid-cols-[max-content_1fr] gap-x-4 gap-y-1">
          <dt>Path</dt><dd className="break-all font-mono">{identity.path}</dd>
          <dt>Container</dt><dd>{identity.container_version ?? "none (bare database)"}</dd>
          <dt>Checksums</dt><dd>{identity.checksums_valid == null ? "not applicable" : identity.checksums_valid ? "valid" : "do not match"}</dd>
          <dt>Database tables</dt><dd>{identity.own_tables} in the file{identity.supplied_tables > 0 ? `, ${identity.supplied_tables} shown read-only from the default database` : ""}</dd>
          <dt>Title ids in path</dt><dd>{identity.path_hints.length ? identity.path_hints.join(", ") : "none"}</dd>
          {selected && <><dt>Evidence</dt><dd>{selected.evidence.join(" ")}</dd></>}
        </dl>
      </details>
    </Card>
  );
}

function CandidateRadio({ c, checked, onChoose }: { c: SaveCandidateDto; checked: boolean; onChoose: (k: string) => void }) {
  return (
    <label className="flex cursor-pointer items-center gap-2 text-sm text-slate-200">
      <input type="radio" name="save-editor-target" checked={checked} onChange={() => onChoose(key(c))} />
      <span>{c.title}</span>
      <span className="text-slate-400">· {c.platform_label}</span>
      {c.exact_schema && <span className="text-[11px] text-emerald-300">exact layout match</span>}
    </label>
  );
}
