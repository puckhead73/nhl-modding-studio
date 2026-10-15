import { Nhl07LeagueStructure } from "../components/Nhl07LeagueStructure";
import { Nhl07CompetitionNames } from "../components/Nhl07CompetitionNames";
import { useState } from "react";
import { Link, useParams } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useIsSaveEditor, useProject } from "../lib/project";
import { FieldRow } from "../components/FieldRow";
import { Card, Pill, Spinner } from "../components/primitives";
import { useCrumbLabel } from "../lib/breadcrumbs";
import type { LeagueViewDto } from "../bindings/LeagueViewDto";
import { DictionaryRenamePanel } from "../components/DictionaryRenamePanel";
import { LeagueScheduleCard } from "../components/LeagueScheduleCard";
import { LeagueLogoCard } from "../components/LeagueLogoCard";
import { DictionaryAttach } from "./Leagues";

/// The League detail page — identity banner, the editable `stockleagues` row (PS4 titles), and
/// the member teams. On titles without a league table the page explains why the name cannot be
/// edited here (the code and its members remain useful) instead of offering a fake edit.
export function LeagueDetail() {
  const { id } = useParams<{ id: string }>();
  const { project } = useProject();
  const saveEditor = useIsSaveEditor();
  const projectId = project!.project_id;
  const code = Number(id);
  // Hooks must run on loading/error renders too. Keep the selected rename key scoped to its league.
  const [groupSelection, setGroupSelection] = useState<{ owner: string; key: string } | null>(null);

  const view = useQuery({
    queryKey: ["league", projectId, code],
    queryFn: () => call("league_view", { projectId, code }),
    enabled: Number.isFinite(code),
  });
  const catalog = useQuery({
    queryKey: ["leagues", projectId],
    queryFn: () => call("list_leagues", { projectId }),
  });
  useCrumbLabel(view.data?.summary.display);

  if (view.isLoading) return <Spinner label="Loading…" />;
  if (view.isError) return <p className="text-rose-400">{String(view.error)}</p>;
  const league = view.data;
  if (!league) return <p className="text-sm text-slate-400">No league with code {code} on this target.</p>;

  const s = league.summary;
  const dict = catalog.data?.dictionary ?? null;
  // PS4: the row's own name key. gen7: the key the vocabulary binds this code to (traced on NHL
  // Legacy Xbox 360's executable, a hypothesis elsewhere) — editable in the panel either way.
  // A group key picked on the structure card ("rename this division") feeds the same rename panel.
  const renameKey = groupSelection?.owner === `${projectId}:${code}` ? groupSelection.key : s.name_key ?? s.dictionary_key ?? "";
  const groupKey = groupSelection?.owner === `${projectId}:${code}` ? groupSelection.key : null;
  const keyEvidence = groupKey != null ? (project?.game_id === "nhl07" ? "hypothesis" : league.structure?.evidence ?? null) : s.name_key != null ? "native" : s.dictionary_key_evidence;
  return (
    <div className="space-y-4">
      <LeagueBanner league={league} />
      <IdentityCard projectId={projectId} league={league} nhl07={project?.game_id === "nhl07"} />
      {project?.game_id === "nhl07" && <Nhl07CompetitionNames projectId={projectId} code={code} dictionarySha={dict?.sha256??null} onEdit={key=>setGroupSelection({owner:`${projectId}:${code}`,key})}/>}
      {project?.game_id === "nhl07" && !dict && <Card title="Language dictionary"><DictionaryAttach projectId={projectId}/></Card>}
      {/* Localization renames write the game's dictionary, not the save: not in Save Editor. */}
      {saveEditor || (project?.game_id === "nhl07" && !groupKey) ? null : dict ? (
        <DictionaryRenamePanel
          key={`${projectId}:${code}:${renameKey}`}
          projectId={projectId}
          initialKey={renameKey}
          keyEvidence={keyEvidence}
          dictionarySha256={dict.sha256}
        />
      ) : (
        <Card title="Rename via dictionary" subtitle="Edit the name shown by the game.">
          <p className="mb-2 text-sm text-slate-300">
            Choose the language dictionary for this game and platform to view and edit league names.
          </p>
          <DictionaryAttach projectId={projectId} />
        </Card>
      )}
      {league.structure && <StructureCard structure={league.structure} onRename={dict && !saveEditor ? (key) => setGroupSelection({ owner: `${projectId}:${code}`, key }) : undefined} />}
      {project?.game_id === "nhl07" && <Nhl07LeagueStructure projectId={projectId} members={league.members}/>}
      {project?.game_id !== "nhl07" && !league.structure && league.structure_blocker && (
        <Card title="Competition structure" subtitle="No group table applies to this session; the team rows' group ids are shown as numbers.">
          <p className="text-sm text-slate-300" data-testid="structure-blocker">
            {league.structure_blocker}
          </p>
        </Card>
      )}
      {!saveEditor && <LeagueLogoCard projectId={projectId} leagueCode={code} />}
      <LeagueScheduleCard projectId={projectId} leagueCode={code} />
      {project?.game_id !== "nhl07" && <MembersCard league={league} />}
    </div>
  );
}

/// Per-row evidence of a group mapping (P1-C): `traced-static` needs no caveat; `inferred` and
/// `unknown` are always shown beside the label.
function EvidencePill({ evidence, note }: { evidence: string; note: string | null }) {
  if (evidence === "traced-static") return null;
  return (
    <Pill tone={evidence === "unknown" ? "error" : "warn"}>
      <span title={note ?? undefined}>{evidence === "unknown" ? "name unknown" : "inferred"}</span>
    </Pill>
  );
}

/// Phase 1.2 — the league's conferences and divisions as this title's own competition table
/// lists them (NHL 12 → Legacy family; PS4 15 → 21 since P1-C). Labels are the attached
/// dictionary's text for each group key; the evidence pill says whether the table was read from
/// this target's executable or is carried by reference from a witnessed twin, and each row
/// carries its own mapping evidence (inferred / unknown rows are never dressed up as names).
function StructureCard({
  structure,
  onRename,
}: {
  structure: NonNullable<LeagueViewDto["structure"]>;
  /// Present when a dictionary is attached: pick a group key for the rename panel below.
  onRename?: (key: string) => void;
}) {
  const shared = new Map(structure.shared_keys.map(([k, others]) => [k, others]));
  const byId = new Map<string, string>(
    [structure.league_group, ...structure.conferences, ...structure.divisions, ...structure.pools, ...structure.other].map((g) => [String(g.id), g.label]),
  );
  const label = (id: bigint) => byId.get(String(id)) ?? `group ${String(id)}`;
  const group = (g: { id: bigint; key: string; label: string; label_source: string; used: boolean; evidence: string; note: string | null; hypothesis_key: string | null }) => (
    <li key={String(g.id)} className="flex flex-wrap items-baseline gap-2">
      <span className="text-slate-200">{g.label}</span>
      <span className="font-mono text-[11px] text-slate-500">
        {g.key ? `${g.key} · ` : ""}id {String(g.id)}
      </span>
      <EvidencePill evidence={g.evidence} note={g.note} />
      {g.label_source === "derived-from-leagues" && (
        <span className="text-[11px] text-slate-500" title="No dictionary key names this group; the label is derived from the leagues whose rows carry it (the current league records)">
          named from its leagues
        </span>
      )}
      {g.hypothesis_key && (
        <span className="text-[11px] text-slate-500" title={g.note ?? undefined}>
          Unverified name mapping: <span className="font-mono">{g.hypothesis_key}</span>
        </span>
      )}
      {!g.used && <Pill tone="neutral">no stock team</Pill>}
      {shared.has(g.key) && (
        <Pill tone="warn">{`key shared with ${shared.get(g.key)!.length} other group${shared.get(g.key)!.length === 1 ? "" : "s"}`}</Pill>
      )}
      {onRename && g.key && (
        <button
          type="button"
          className="text-[11px] text-sky-400 underline-offset-2 hover:underline"
          onClick={() => onRename(g.key)}
          title={shared.has(g.key) ? `Renaming this key also renames: ${shared.get(g.key)!.join(", ")}` : "Rename via the dictionary panel below"}
        >
          rename…
        </button>
      )}
    </li>
  );
  return (
    <Card
      title="Competition structure"
      subtitle="Conferences, divisions and pools for this league."
    >
      <div className="mb-2 flex items-center gap-2 text-xs text-slate-400">
        <Pill tone={structure.evidence === "traced" ? "ok" : "warn"}>
          group table {structure.evidence === "traced" ? "traced on this target" : "by reference — this target's executable not inspected"}
        </Pill>
        <span>
          league group <span className="font-mono">{String(structure.league_group.id)}</span> ({structure.league_group.label})
        </span>
        <EvidencePill evidence={structure.league_group.evidence} note={structure.league_group.note} />
      </div>
      {structure.league_group.hypothesis_key && (
        <p className="mb-2 text-xs text-slate-500" data-testid="structure-hypothesis">
          The executable names no key for this group; the research hypothesis{" "}
          <span className="font-mono">{structure.league_group.hypothesis_key}</span> is a hint, not the name. {structure.league_group.note}
        </p>
      )}
      <details className="mt-2 text-xs text-slate-400"><summary className="cursor-pointer">Mapping details</summary>
      {structure.provenance && (
        <p className="mb-2 text-xs text-slate-500" data-testid="structure-provenance">
          Title provenance: {structure.provenance}
        </p>
      )}
      </details>
      {structure.shared_with.length > 0 && (
        <p className="mb-2 text-xs text-slate-400" data-testid="structure-shared-with">
          This block is shared with {structure.shared_with.map(([c, name]) => `${name} (code ${String(c)})`).join(", ")}: their teams carry the same group ids.
        </p>
      )}
      {structure.conferences.length === 0 && structure.divisions.length === 0 && structure.pools.length === 0 ? (
        <p className="text-sm text-slate-300">Single group: the league's own id fills the conference and division fields.</p>
      ) : structure.conferences.length === 0 && structure.divisions.length === 0 ? (
        <div>
          <div className="mb-1 text-xs font-semibold text-slate-300">Pools (carried in both the conference and the division field)</div>
          <ul className="space-y-1 text-sm">{structure.pools.map(group)}</ul>
        </div>
      ) : (
        <div className="grid gap-4 sm:grid-cols-2">
          <div>
            <div className="mb-1 text-xs font-semibold text-slate-300">Conferences</div>
            <ul className="space-y-1 text-sm">{structure.conferences.map(group)}</ul>
          </div>
          <div>
            <div className="mb-1 text-xs font-semibold text-slate-300">Divisions</div>
            <ul className="space-y-1 text-sm">{structure.divisions.map(group)}</ul>
          </div>
        </div>
      )}
      {structure.conferences.length + structure.divisions.length > 0 && structure.pools.length > 0 && (
        <div className="mt-2">
          <div className="mb-1 text-xs font-semibold text-slate-300">Pools</div>
          <ul className="space-y-1 text-sm">{structure.pools.map(group)}</ul>
        </div>
      )}
      {structure.pairs.length > 1 && (
        <p className="mt-2 text-xs text-slate-500" data-testid="structure-pairs">
          Arrangements the stock rows carry (conference → division):{" "}
          {structure.pairs.map(([c, d]) => `${label(c)} → ${label(d)}`).join(" · ")}
        </p>
      )}
      {structure.exceptional.length > 0 && (
        <details className="mt-2 text-xs text-slate-500" data-testid="structure-exceptional">
          <summary className="cursor-pointer">Exceptional stock arrangements ({structure.exceptional.length}) — recorded, never offered</summary>
          <ul className="mt-1 list-disc space-y-1 pl-5">
            {structure.exceptional.map((x, i) => (
              <li key={i}>{x}</li>
            ))}
          </ul>
        </details>
      )}
      {structure.witness_notes.length > 0 && (
        <ul className="mt-2 list-disc space-y-1 pl-5 text-xs text-slate-500" data-testid="structure-witness-notes">
          {structure.witness_notes.map((n, i) => (
            <li key={i}>{n}</li>
          ))}
        </ul>
      )}
      {structure.other.length > 0 && (
        <details className="mt-2 text-xs text-slate-500">
          <summary className="cursor-pointer">Other groups in this block ({structure.other.length})</summary>
          <ul className="mt-1 space-y-1 text-sm">{structure.other.map(group)}</ul>
        </details>
      )}
    </Card>
  );
}

/// The code -> dictionary-key evidence class as resolved for *this* target (never a neighbour's):
/// `traced` only where this target's own executable table was read; `by-reference` where the
/// mapping is inherited from a witnessed target (same database and key set, executable not
/// inspected); `hypothesis` where only the key family is present in the dictionary.
function describeKeyEvidence(evidence: string | null): string {
  switch (evidence) {
    case "traced":
      return "binding traced on this target";
    case "by-reference":
      return "binding by reference — this target's executable not inspected";
    case "hypothesis":
      return "binding hypothesis";
    case null:
      return "binding unstated";
    default:
      return `binding ${evidence}`;
  }
}

function LeagueBanner({ league }: { league: LeagueViewDto }) {
  const s = league.summary;
  const isKey = s.label == null && s.name_key != null;
  return (
    <div className="rounded-lg border border-slate-800 bg-gradient-to-r from-slate-900/80 to-slate-900/30 p-4">
      <div className="flex flex-wrap items-center gap-2">
        <h1
          className={
            "text-2xl font-semibold leading-tight " +
            (isKey || s.label == null ? "font-mono text-amber-200" : "text-slate-50")
          }
        >
          {s.display}
        </h1>
        <Pill tone="neutral">code {s.code}</Pill>
        {s.label != null && s.name_key != null && (
          <Pill tone="neutral">
            key <span className="font-mono">{s.name_key}</span>
          </Pill>
        )}
        {s.name_key == null && s.dictionary_key != null && (
          <Pill tone={s.dictionary_key_evidence === "traced" ? "ok" : "warn"}>
            key <span className="font-mono">{s.dictionary_key}</span> ({describeKeyEvidence(s.dictionary_key_evidence)})
          </Pill>
        )}
        {s.confidence && <Pill tone={s.confidence === "candidate" ? "warn" : "neutral"}>{s.confidence}</Pill>}
        <Pill tone="neutral">
          {s.member_teams} team{s.member_teams === 1 ? "" : "s"}
        </Pill>
      </div>
      <details className="mt-2 text-xs text-slate-400"><summary className="cursor-pointer">Name source</summary><p className="mt-1">
        {isKey
          ? "Shown as its localization key: the game looks this key up in its language dictionary; the on-screen name is not stored in the database."
          : s.evidence}
      </p></details>
    </div>
  );
}

function IdentityCard({ projectId, league, nhl07 }: { projectId: number; league: LeagueViewDto; nhl07?:boolean }) {
  const s = league.summary;
  if(nhl07) return <Card title="Identity"><dl className="grid gap-3 text-sm sm:grid-cols-3">
    <div><dt className="text-slate-400">Name</dt><dd>{s.display}</dd></div>
    <div><dt className="text-slate-400">Teams</dt><dd>{s.member_teams}</dd></div>
    <div><dt className="text-slate-400">Name source</dt><dd>{s.evidence.includes("dictionary text for key")?"Project dictionary":"Stock team membership"}</dd></div>
  </dl></Card>;
  if (league.name_edit_blocker || s.record == null) {
    return (
      <Card title="Identity">
        <p className="text-sm text-slate-300">{league.name_edit_blocker ?? "This league has no editable row on this target."}</p>
        <p className="mt-2 text-xs text-slate-500">
          Native code {s.code}
          {s.label ? ` · label ${s.label}` : ""} · {s.evidence}
        </p>
      </Card>
    );
  }
  const name = league.fields.find((f) => f.name === "name");
  const rest = league.fields.filter((f) => f !== name);
  return (
    <Card
      title="League details"
      subtitle={
        league.name_max_bytes != null
          ? `The name is a localization key of at most ${league.name_max_bytes} bytes; the key must exist in the game's dictionary to display as text. The league id is the primary key and cannot be changed here.`
          : undefined
      }
    >
      <div className="sm:grid sm:grid-cols-2 sm:gap-x-6">
        {name && <FieldRow projectId={projectId} record={s.record} field={name} label="Name key" />}
        {rest.map((f) => (
          <FieldRow key={f.studio_field} projectId={projectId} record={s.record!} field={f} label={({ leagueid: "League ID", artid: "Artwork ID", renderid: "Render ID", orderid: "Display order", primarycolor: "Primary colour", secondarycolor: "Secondary colour" } as Record<string, string>)[f.name]} />
        ))}
      </div>
      <p className="mt-2 text-xs text-slate-500">
        Art, render and order ids and the colours are presentation values whose consumers are not yet
        traced; they are shown raw and edit as plain numbers.
      </p>
    </Card>
  );
}

function MembersCard({ league }: { league: LeagueViewDto }) {
  return (
    <Card title={`Member teams (${league.members.length})`}>
      {league.members.length === 0 ? (
        <p className="text-sm text-slate-500">No team carries this league code.</p>
      ) : (
        <ul className="grid gap-x-6 gap-y-1 sm:grid-cols-2">
          {league.members.map((t) => (
            <li key={t.team_id} className="flex items-center gap-2 text-sm">
              <span className="w-12 shrink-0 font-medium text-teal-300/80">{t.abbrev}</span>
              <Link className="truncate text-sky-400 hover:underline" to={`/app/database/teams/${t.team_id}`}>
                {t.name || `Team #${t.team_id}`}
              </Link>
            </li>
          ))}
        </ul>
      )}
    </Card>
  );
}
