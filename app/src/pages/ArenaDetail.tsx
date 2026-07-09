import { type ReactNode } from "react";
import { Link, useParams } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import { EditableField } from "../components/EditableField";
import { FieldRow } from "../components/FieldRow";
import { Pill, Spinner } from "../components/primitives";
import { useCrumbLabel } from "../lib/breadcrumbs";
import type { FieldViewDto } from "../bindings/FieldViewDto";

/// The Arena detail — the reference layout: identity header with capacity / ice-type / tenant
/// pills, the editable Identity and Presentation & Audio sections, the raw-field disclosure, and
/// the "Teams playing here" join (every `kOtt.buDa` pointing at this arena, resolved to names).
///
/// The reference app carried several StEO fields as unanchored candidates ("Atmosphere",
/// "Stock Flag", "Regional Group") — our anchor metadata names them outright (`avgattendance`,
/// `stanchion`, `arenaartid`, plus the audio ids), superseding those readings, so everything
/// here is a named, editable cell rather than a raw hex dump.
export function ArenaDetail() {
  const { id } = useParams<{ id: string }>();
  const { project } = useProject();
  const projectId = project!.project_id;
  const arenaId = Number(id);

  const view = useQuery({
    queryKey: ["arena", projectId, arenaId],
    queryFn: () => call("arena_view", { projectId, arenaId }),
    enabled: Number.isFinite(arenaId),
  });
  useCrumbLabel(view.data?.all_fields.find((f) => f.name === "arenaname")?.display.trim());

  if (view.isLoading) return <Spinner label="Loading arena…" />;
  if (view.isError) return <p className="text-rose-400">{String(view.error)}</p>;
  const arena = view.data;
  if (!arena) return <p className="text-sm text-slate-400">Select an arena.</p>;

  const byName = (name: string): FieldViewDto | undefined =>
    arena.all_fields.find((f) => f.name === name);
  const name = byName("arenaname")?.display.trim() || `Arena #${arena.arena_id}`;
  const city = byName("cityname")?.display.trim();
  const capacity = byName("maxattendance")?.decoded;
  const iceType = byName("arenatype")?.enum_label;

  // An editable cell by metadata name; hidden when this title's schema lacks the field.
  const F = ({ name, label, tip }: { name: string; label: string; tip?: string }) => {
    const f = byName(name);
    return f ? (
      <div title={tip}>
        <EditableField projectId={projectId} record={arena.record} field={f} label={label} />
      </div>
    ) : null;
  };

  return (
    <div className="max-w-5xl space-y-3">
      {/* Header — identity + at-a-glance pills, mirroring the player/team banners. */}
      <div className="rounded-lg border border-slate-800 bg-gradient-to-r from-slate-900/80 to-slate-900/30 p-4">
        <div className="flex flex-wrap items-baseline gap-3">
          <h1 className="text-2xl font-semibold leading-tight text-slate-50">{name}</h1>
          <span className="text-xs text-slate-500">ID #{arena.arena_id}</span>
        </div>
        {city && <p className="mt-1 text-xs text-slate-400">{city}</p>}
        <div className="mt-3 flex flex-wrap items-center gap-1.5">
          {capacity != null && <Pill tone="ok">{capacity.toLocaleString()} seats</Pill>}
          {iceType && <Pill>{iceType}</Pill>}
          {arena.teams.length > 0 ? (
            <Pill>
              {arena.teams.length} team{arena.teams.length === 1 ? "" : "s"}
            </Pill>
          ) : (
            <Pill tone="warn">Unused</Pill>
          )}
        </div>
      </div>

      <div className="space-y-6 rounded-lg border border-slate-800 bg-slate-900/40 p-4">
        <Section title="Identity">
          <F name="arenaname" label="Name" />
          <F name="cityname" label="City" />
          <F
            name="maxattendance"
            label="Capacity"
            tip="StEO.maxattendance — seating capacity (the FK target teams reference via kOtt.buDa is the Arena ID, not this)."
          />
          <F
            name="arenatype"
            label="Ice Surface"
            tip="StEO.arenatype — NHL ice (200×85) vs international ice (200×100). Anchored: every NA arena carries 0, every European arena 1."
          />
        </Section>

        <Section title="Presentation">
          <F
            name="avgattendance"
            label="Avg Attendance"
            tip="StEO.avgattendance (7-bit). NHL arenas cluster 80–100, AHL 60–95, CHL 44–90 — reads as a percentage. The reference app carried this as a candidate 'atmosphere rating'; the metadata name supersedes that."
          />
          <F
            name="stanchion"
            label="Stanchion"
            tip="StEO.stanchion (1-bit). Set on the 30 NHL home venues (+2 specials) in the anchor. The reference app read it as a 'Main NHL Arena' flag; the metadata names it stanchion (glass-post style)."
          />
          <F name="centerline" label="Center Line" tip="StEO.centerline (6-bit) — art variant." />
          <F
            name="centericelogo"
            label="Center Ice Logo"
            tip="StEO.centericelogo — art asset id (all-zero across the Legacy anchor)."
          />
          <F
            name="arenaartid"
            label="Arena Art ID"
            tip="StEO.arenaartid — the arena's art asset id. The reference app carried this as a candidate 'regional group'; the metadata name supersedes that."
          />
          <F name="powerring" label="Power Ring" tip="StEO.powerring — art asset id." />
          <F name="puck" label="Puck" tip="StEO.puck — puck art asset id." />
          <F name="bouncemap" label="Bounce Map" tip="StEO.bouncemap — board-physics map id." />
          <F name="index" label="Self Index" tip="StEO.index — record-index echo; edit with care." />
        </Section>

        <Section title="Audio">
          <F name="goalhorn" label="Goal Horn" tip="StEO.goalhorn — audio id." />
          <F name="periodhorn" label="Period Horn" tip="StEO.periodhorn — audio id (the field the reference app surfaced as 'Raw ClpB'; its duplicate values across arenas are shared horn samples)." />
          <F name="chantaudioid" label="Chant Audio" tip="StEO.chantaudioid — crowd-chant sample id." />
          <F name="cityaudioid" label="City Audio" tip="StEO.cityaudioid — commentary city-name sample id." />
          <F name="arenaaudioid" label="Arena Audio" tip="StEO.arenaaudioid — commentary arena-name sample id." />
        </Section>
      </div>

      {/* Teams playing here — the kOtt.buDa → StEO join, resolved server-side. */}
      <section className="rounded-lg border border-slate-800 bg-slate-900/40">
        <header className="flex items-baseline justify-between gap-3 border-b border-slate-800 px-4 py-2">
          <h2 className="text-sm font-semibold text-slate-100">Teams playing here</h2>
          <span className="text-[11px] text-slate-500">
            {arena.teams.length === 0
              ? "no teams"
              : `${arena.teams.length} team${arena.teams.length === 1 ? "" : "s"}`}
          </span>
        </header>
        {arena.teams.length === 0 ? (
          <div className="px-4 py-6 text-center text-xs text-slate-500">
            No team references this arena as its home. Unassigned arenas are usually neutral-site
            venues or unused alternates.
          </div>
        ) : (
          <ul className="divide-y divide-slate-800/60">
            {arena.teams.map((t) => (
              <li key={t.team_id}>
                <Link
                  to={`/app/database/teams/${t.team_id}`}
                  className="flex items-center gap-3 px-4 py-2 transition-colors hover:bg-slate-800/40"
                >
                  <span className="font-mono text-[10px] tabular-nums text-slate-500">
                    #{t.team_id}
                  </span>
                  <span className="min-w-0 flex-1 truncate text-xs text-slate-100">
                    {t.name.trim() || `Team #${t.team_id}`}
                  </span>
                  {t.abbrev && (
                    <span className="inline-block rounded bg-slate-800/60 px-1.5 py-0.5 font-mono text-[10px] text-slate-200">
                      {t.abbrev}
                    </span>
                  )}
                </Link>
              </li>
            ))}
          </ul>
        )}
      </section>

      <details className="rounded-md border border-slate-800 bg-slate-900/40 px-3 py-2">
        <summary className="cursor-pointer text-xs font-semibold text-slate-300">
          All arena fields ({arena.all_fields.length}) — full editor
        </summary>
        <div className="mt-2 sm:grid sm:grid-cols-2 sm:gap-x-6">
          {arena.all_fields.map((f) => (
            <FieldRow key={f.studio_field} projectId={projectId} record={arena.record} field={f} />
          ))}
        </div>
      </details>
    </div>
  );
}

/** A titled block whose cells flow in a 4-column grid (the Player/Team Data section style). */
function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section>
      <h3 className="mb-3 border-b border-slate-800 pb-1 text-[11px] font-bold uppercase tracking-wider text-slate-400">
        {title}
      </h3>
      <div className="grid grid-cols-2 items-start gap-x-8 gap-y-3 sm:grid-cols-3 lg:grid-cols-4">
        {children}
      </div>
    </section>
  );
}
