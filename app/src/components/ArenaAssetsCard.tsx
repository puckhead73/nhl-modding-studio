// Phase 2.5 (22 Sep 2026) — the read-only "Arena assets" card on the Arena page.
//
// Shows what the backend measured for this arena's shipped files: the title's contract, each asset
// family with its binding class — established (the arena's art id names the file, research R62),
// via the home team (the centre-ice logo and banners are team-keyed), candidate (the bounce map)
// or no established binding (the other ice layers) — every resolved file with present / missing /
// unknown evidence and a thumbnail when it can be previewed, the spare shipped venues, and on PS4
// the venue directories with the vault reference named as the (unread) selector. Nothing writes.
//
// Result ownership: keyed on project + arena + install root + platform; a payload is rendered only
// when its `arena_id` is the arena the card shows.

import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useAssetRootArgs } from "../lib/project";
import type { ArenaAssetsDto } from "../bindings/ArenaAssetsDto";
import type { AssetFamilyDto } from "../bindings/AssetFamilyDto";
import { TextureThumbnail } from "./TextureReplace";
import { Card, Spinner } from "./primitives";

const STATUS_TONE: Record<string, string> = {
  present: "border-emerald-700/60 bg-emerald-900/30 text-emerald-200",
  missing: "border-rose-700/60 bg-rose-900/30 text-rose-200",
  unknown: "border-amber-700/60 bg-amber-900/30 text-amber-200",
};

const BINDING: Record<string, { label: string; tone: string }> = {
  established: { label: "established", tone: "border-sky-700/60 bg-sky-900/30 text-sky-200" },
  via_home_team: { label: "via home team", tone: "border-violet-700/60 bg-violet-900/30 text-violet-200" },
  candidate: { label: "candidate key", tone: "border-amber-700/60 bg-amber-900/30 text-amber-200" },
  no_established_binding: { label: "no established binding", tone: "border-slate-600 bg-slate-800/60 text-slate-300" },
};

function Badge({ tone, children, testId, title }: { tone: string; children: React.ReactNode; testId?: string; title?: string }) {
  return (
    <span className={"rounded border px-1.5 py-0.5 text-[10px] font-semibold uppercase tracking-wide " + tone} data-testid={testId} title={title}>
      {children}
    </span>
  );
}

/** Only texture families can be thumbnailed through `texture_preview`; a venue model container is a mesh package. */
const PREVIEWABLE = new Set(["jumbotron", "centre_ice_logo", "banners", "bounce_map"]);

export function ArenaAssetsCard({ projectId, arenaId }: { projectId: number; arenaId: number }) {
  const { installDir, rootArgs } = useAssetRootArgs();
  const q = useQuery({
    queryKey: ["arenaAssets", projectId, arenaId, installDir ?? "", rootArgs.platform ?? "", rootArgs.rendering_override ?? ""],
    queryFn: () => call("arena_assets", { projectId, roots: rootArgs, arenaId }),
    enabled: Number.isFinite(arenaId),
    staleTime: Infinity,
    retry: false,
  });
  const [spares, setSpares] = useState<string | null>(null);

  if (q.isError) {
    return (
      <Card title="Arena assets">
        <p className="text-xs text-rose-300" data-testid="arena-assets-error">
          {String(q.error)}
        </p>
      </Card>
    );
  }
  const d: ArenaAssetsDto | undefined = q.data && q.data.arena_id === arenaId ? q.data : undefined;
  if (q.data === null) {
    return (
      <Card title="Arena assets">
        <p className="text-xs text-slate-400" data-testid="arena-assets-none">
          This arena id is not in the table.
        </p>
      </Card>
    );
  }
  if (!d) return <Spinner label="Reading this arena's shipped assets…" />;

  return (
    <Card
      title="Arena assets"
      subtitle="Shipped files joined to this arena only where the research measured the join; everything else is listed as shipped, with its key named a candidate or unresolved. Read-only."
    >
      <div className="space-y-2 text-xs" data-testid="arena-assets-card" data-arena-id={d.arena_id}>
        <p className="flex flex-wrap items-center gap-2">
          <Badge
            tone={d.contract_status === "gen7" ? BINDING.established.tone : BINDING.candidate.tone}
            testId="arena-assets-contract-status"
          >
            {d.contract_status === "gen7" ? "gen7 file-name rule" : d.contract_status === "ps4_vault_reference" ? "PS4 · vault reference" : "not measured"}
          </Badge>
          {d.arena_art_id != null ? <span className="text-slate-400">arena art id {d.arena_art_id}</span> : null}
        </p>
        <p className="text-slate-400" data-testid="arena-assets-contract" title={d.contract}>
          {d.contract}
        </p>

        {d.families.map((f) => (
          <Family key={f.family} f={f} installDir={installDir} sparesOpen={spares === f.family} onToggleSpares={() => setSpares((s) => (s === f.family ? null : f.family))} />
        ))}

        {d.ps4_venue ? <Ps4Venue l={d.ps4_venue} artId={d.arena_art_id} /> : null}

        {d.venue_directories.length > 0 ? (
          <div data-testid="arena-assets-venue-dirs">
            <p className="text-slate-300">
              {d.venue_directories.length} shipped venue directories under <code>stadiums/</code> — no arena column names them; the join above runs through the vault level record.
            </p>
            <p className="text-slate-500">
              {d.venue_directories.map(([n, m]) => `${n} (${m})`).join(" · ")}
            </p>
          </div>
        ) : null}

        {d.notes.length > 0 ? (
          <ul className="list-disc space-y-0.5 pl-4 text-slate-500" data-testid="arena-assets-notes">
            {d.notes.map((n, i) => (
              <li key={i}>{n}</li>
            ))}
          </ul>
        ) : null}
      </div>
    </Card>
  );
}

function Family({ f, installDir, sparesOpen, onToggleSpares }: { f: AssetFamilyDto; installDir: string | null; sparesOpen: boolean; onToggleSpares: () => void }) {
  const b = BINDING[f.binding] ?? BINDING.no_established_binding;
  return (
    <section className="rounded-md border border-slate-800 bg-slate-950/40 px-3 py-2" data-testid={`arena-family-${f.family}`}>
      <p className="flex flex-wrap items-center gap-2">
        <span className="font-semibold text-slate-100">{f.label}</span>
        <Badge tone={b.tone} testId={`arena-family-binding-${f.family}`} title={f.rule}>
          {b.label}
        </Badge>
        {f.shipped_files != null ? <span className="text-slate-500">{f.shipped_files} shipped</span> : null}
      </p>
      <p className="text-slate-500" title={f.rule}>
        {f.rule}
      </p>
      {f.files.length > 0 ? (
        <ul className="mt-1 space-y-1">
          {f.files.map((x) => (
            <li key={`${x.key_source}-${x.file_stem}`} className="flex items-center gap-2" data-testid={`arena-file-${f.family}-${x.file_stem}`} title={x.reason ?? x.path ?? undefined}>
              {installDir && x.status === "present" && x.rel_path && PREVIEWABLE.has(f.family) ? (
                <TextureThumbnail installDir={installDir} relPath={x.rel_path} className="h-8 w-8 rounded" />
              ) : (
                <span className="inline-block h-8 w-8 rounded border border-dashed border-slate-700" aria-hidden />
              )}
              <Badge tone={STATUS_TONE[x.status] ?? ""} testId={`arena-file-status-${f.family}-${x.file_stem}`}>
                {x.status}
              </Badge>
              <code className="truncate text-slate-300">{x.rel_path ?? x.file_stem}</code>
              <span className="truncate text-slate-500">— {x.key_source}</span>
              {x.reason ? <span className="truncate text-slate-500">— {x.reason}</span> : null}
            </li>
          ))}
        </ul>
      ) : null}
      {f.spare_keys.length > 0 ? (
        <div className="mt-1">
          <button type="button" className="rounded border border-slate-700 px-2 py-0.5 text-slate-300 hover:bg-slate-800" onClick={onToggleSpares} data-testid={`arena-spares-toggle-${f.family}`}>
            {f.spare_keys.length} shipped {f.spare_keys.length === 1 ? "id" : "ids"} no arena row names {sparesOpen ? "▾" : "▸"}
          </button>
          {sparesOpen ? (
            <p className="text-slate-500" data-testid={`arena-spares-${f.family}`}>
              {f.spare_keys.join(", ")}
            </p>
          ) : null}
        </div>
      ) : null}
      {f.notes.length > 0 ? (
        <ul className="mt-1 list-disc pl-4 text-slate-500" data-testid={`arena-family-notes-${f.family}`}>
          {f.notes.map((n, i) => (
            <li key={i}>{n}</li>
          ))}
        </ul>
      ) : null}
    </section>
  );
}

const VENUE_CLASS_LABEL: Record<string, string> = {
  unambiguous: "one level, one folder",
  ambiguous: "several levels share this id — none chosen",
  level_without_folder: "level without a shipped folder",
  no_level: "no level record carries this id",
  unmeasured: "no level table for this title",
};

const FOLDER_LABEL: Record<string, string> = {
  present: "folder present",
  missing: "folder missing from the attached tree",
  no_folder_shipped: "no folder shipped",
  unknown: "no tree attached",
};

/** The PS4 vault-level listing: every candidate level for the arena's art id with its
 *  folder status, texpack keys and rink id; the class and its reason; shared content once; the
 *  dependency cascade stated in place of any re-point affordance. Read-only. */
function Ps4Venue({ l, artId }: { l: NonNullable<ArenaAssetsDto["ps4_venue"]>; artId: number | null }) {
  return (
    <section className="space-y-1 rounded-md border border-slate-800 bg-slate-950/40 px-3 py-2" data-testid="arena-venue-levels" data-class={l.class}>
      <p className="text-slate-200">
        Venue by <span title={l.key_label}>vault level name</span>
        {artId != null ? <span className="text-slate-500"> · arena art id {artId}</span> : null}
        {" · "}
        <span className={l.class === "unambiguous" ? "text-emerald-300" : "text-amber-300"} data-testid="arena-venue-class">
          {VENUE_CLASS_LABEL[l.class] ?? l.class}
        </span>
      </p>
      <p className="text-slate-500" data-testid="arena-venue-reason">
        {l.reason}
      </p>
      {l.candidates.length > 0 ? (
        <ul className="space-y-0.5" data-testid="arena-venue-candidates">
          {l.candidates.map((c) => (
            <li key={c.level_name} className="flex flex-wrap items-baseline gap-2" data-testid={`arena-venue-level-${c.level_name}`} data-folder={c.folder}>
              <code className="text-slate-300">{c.level_name}</code>
              <span className="text-slate-400">{c.dir ?? "—"}</span>
              <span className={c.folder === "present" ? "text-emerald-300" : "text-amber-300/80"}>
                {FOLDER_LABEL[c.folder] ?? c.folder}
                {c.members != null ? ` (${c.members} files)` : ""}
              </span>
              <span className="text-slate-500">
                {c.texpacks.map((t) => `${t.family.replace("_", " ")} ${t.key}${t.in_set ? "" : " (no texpack with this key)"}`).join(" · ")}
                {c.rink_id != null ? ` · rink ${c.rink_id}` : ""}
              </span>
            </li>
          ))}
        </ul>
      ) : null}
      {l.shared_content.length > 0 ? (
        <p className="text-slate-500" data-testid="arena-venue-shared">
          Shared / unreferenced content: {l.shared_content.map(([n, m]) => (m != null ? `${n} (${m})` : n)).join(" · ")}
        </p>
      ) : null}
      {l.dependencies ? <p className="text-slate-500">{l.dependencies}</p> : null}
      <p className="text-slate-500" data-testid="arena-venue-reassignment">
        Re-pointing is not offered: {l.reassignment_reason}
      </p>
    </section>
  );
}
