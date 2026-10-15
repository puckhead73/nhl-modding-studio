import { useEffect, useRef, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import type { MembershipOptionsDto } from "../bindings/MembershipOptionsDto";
import type { MembershipResultDto } from "../bindings/MembershipResultDto";
import { Nhl07MembershipCard } from "./Nhl07MembershipCard";
import { Button, Card, Pill } from "./primitives";

/// Phase 1.2 — move a team to another league with the conference / division that league uses on
/// *this* title. The choices and their labels come from the server (the title's own competition
/// table when it has one — NHL 12 → Legacy and, since P1-C, PS4 15 → 21 — resolved through the
/// attached dictionary); the panel never invents an id. Each choice carries the evidence of its
/// mapping: `inferred` and `unknown` rows are marked, an unknown group is offered by its number
/// only. On titles without a fitting table (NHL 08–11 local numbers, a PS4 database no table
/// describes) the extra fields are plain numbers and the card says why. Every async completion is
/// bound to the team and project it started for.
export function LeagueMembershipCard(props: { projectId: number; teamId: number }) {
  const {project}=useProject();
  return project?.game_id === "nhl07" ? <Nhl07MembershipCard key={`${props.projectId}/${props.teamId}`} {...props}/> : <LegacyMembershipCard {...props}/>;
}
function LegacyMembershipCard({ projectId, teamId }: { projectId: number; teamId: number }) {
  const qc = useQueryClient();
  const { update } = useProject();
  const opts = useQuery({
    queryKey: ["membership", projectId, teamId],
    queryFn: () => call("team_membership_options", { projectId, teamId }),
  });
  const [league, setLeague] = useState<number | null>(null);
  const [conference, setConference] = useState<string>("");
  const [division, setDivision] = useState<string>("");
  const [result, setResult] = useState<MembershipResultDto | null>(null);
  const [summaryError, setSummaryError] = useState<string | null>(null);
  const genRef = useRef(0);
  const mountedRef = useRef(true);
  // The context the card is showing *now* — read after every await, never the captured one, so a
  // completion for an earlier project/team can neither show a result nor push that project's
  // summary into the global state (R1 F3; the same rule as the schedule card).
  const ctxRef = useRef({ projectId, teamId });
  ctxRef.current = { projectId, teamId };
  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      genRef.current += 1;
    };
  }, []);
  // A new team or project: the held selection and result belong to the old one.
  useEffect(() => {
    genRef.current += 1;
    setLeague(null);
    setConference("");
    setDivision("");
    setResult(null);
    setSummaryError(null);
  }, [projectId, teamId]);

  const o: MembershipOptionsDto | undefined = opts.data;
  const currentLeague = league ?? (o ? Number(o.current.league_code) : null);
  const groups = o?.groups_by_league.find((g) => Number(g.league_code) === currentLeague);
  const singleGroup = !!groups && groups.conferences.length === 0 && groups.divisions.length === 0;
  // A league whose block is established but whose target league group has no known name
  // (PS4 NHL 19–21 ids 77/78): the move writes the id the rows already use, and says so.
  const targetUnknown = !!groups && groups.league_group_evidence === "unknown";
  const targetInferred = !!groups && groups.league_group_evidence === "inferred";
  // R1 F1: only the (conference, division) arrangements the title's stock rows carry are
  // offered — the division list follows the chosen conference through `pairs`.
  const pairs = groups?.pairs ?? [];
  const divisionsFor = (conf: string) =>
    conf === "" ? [] : (groups?.divisions ?? []).filter((d) => pairs.some(([c, dv]) => Number(c) === Number(conf) && Number(dv) === Number(d.value)));
  const divisionOptions = divisionsFor(conference);

  const move = useMutation({
    mutationFn: async () => {
      const gen = genRef.current;
      const forTeam = teamId;
      const forProject = projectId;
      const num = (s: string) => (s.trim() === "" ? null : Number(s));
      const r = await call("set_team_membership", {
        projectId: forProject,
        teamId: forTeam,
        leagueCode: currentLeague!,
        conference: groups ? (singleGroup ? null : num(conference)) : num(conference),
        division: groups ? (singleGroup ? null : num(division)) : num(division),
      });
      return { r, gen, forTeam, forProject };
    },
    onSuccess: async ({ r, gen, forTeam, forProject }) => {
      void qc.invalidateQueries();
      const same = () => mountedRef.current && gen === genRef.current && ctxRef.current.projectId === forProject && ctxRef.current.teamId === forTeam;
      if (same()) {
        setResult(r);
        setLeague(null);
        setConference("");
        setDivision("");
      }
      let summary;
      try {
        summary = await call("project_summary", { projectId: forProject });
      } catch (e) {
        // A late summary failure belongs to the context that started it; nothing global changes.
        if (same()) setSummaryError(String(e));
        return;
      }
      // The summary is that project's: apply it only while that project is still the one open.
      if (mountedRef.current && ctxRef.current.projectId === forProject) update(summary);
    },
  });

  if (opts.isLoading) return null;
  if (opts.isError) return <p className="text-sm text-rose-400">{String(opts.error)}</p>;
  if (!o) return null;
  if (o.blocker) {
    return (
      <Card title="League membership" subtitle="Not available on this title.">
        <p className="text-sm text-slate-300">{o.blocker}</p>
      </Card>
    );
  }
  const cur = o.current;
  const canMove = currentLeague != null && !move.isPending && (!groups || singleGroup || (conference !== "" && division !== ""));

  return (
    <Card
      title="League membership"
      subtitle="Move this team to another league; conference and division come from this title's own competition table where it has one."
    >
      <div className="mb-2 flex flex-wrap items-center gap-2 text-xs text-slate-400" data-testid="membership-current">
        <span>
          now: <span className="text-slate-200">{cur.league_label}</span>
          {cur.conference && <> · {cur.conference.label}<ChoiceEvidence evidence={cur.conference.evidence} /></>}
          {cur.division && <> · {cur.division.label}<ChoiceEvidence evidence={cur.division.evidence} /></>}
        </span>
        <Pill tone={o.group_evidence === "traced" ? "ok" : o.group_evidence ? "warn" : "neutral"}>
          {o.model}
          {o.group_evidence ? ` · ${o.group_evidence}` : ""}
        </Pill>
      </div>
      <div className="grid gap-2 sm:grid-cols-3">
        <label className="text-xs text-slate-400">
          League
          <select
            aria-label="target league"
            className="mt-1 w-full rounded-md border border-slate-700 bg-slate-950 px-2 py-1 text-sm text-slate-100"
            value={currentLeague ?? ""}
            onChange={(e) => {
              setLeague(Number(e.target.value));
              setConference("");
              setDivision("");
              setResult(null);
            }}
          >
            {o.leagues.map((l) => (
              <option key={String(l.value)} value={Number(l.value)}>
                {l.label}
              </option>
            ))}
          </select>
        </label>
        {groups && !singleGroup ? (
          <>
            <label className="text-xs text-slate-400">
              Conference
              <select
                aria-label="conference"
                className="mt-1 w-full rounded-md border border-slate-700 bg-slate-950 px-2 py-1 text-sm text-slate-100"
                value={conference}
                onChange={(e) => {
                  const next = e.target.value;
                  setConference(next);
                  // A division that is not paired with the new conference cannot stay selected.
                  if (!divisionsFor(next).some((d) => String(Number(d.value)) === division)) setDivision("");
                }}
              >
                <option value="">— choose —</option>
                {groups.conferences.map((c) => (
                  <option key={String(c.value)} value={Number(c.value)}>
                    {choiceText(c)}
                  </option>
                ))}
              </select>
            </label>
            <label className="text-xs text-slate-400">
              Division
              <select
                aria-label="division"
                className="mt-1 w-full rounded-md border border-slate-700 bg-slate-950 px-2 py-1 text-sm text-slate-100"
                value={division}
                onChange={(e) => setDivision(e.target.value)}
                disabled={conference === ""}
              >
                <option value="">{conference === "" ? "— choose a conference first —" : "— choose —"}</option>
                {divisionOptions.map((d) => (
                  <option key={String(d.value)} value={Number(d.value)}>
                    {choiceText(d)}
                  </option>
                ))}
              </select>
            </label>
            {groups.exceptional.length > 0 && (
              <details className="text-xs text-slate-500 sm:col-span-3" data-testid="membership-exceptional">
                <summary className="cursor-pointer">
                  {groups.exceptional.length} stock arrangement{groups.exceptional.length === 1 ? "" : "s"} not offered (special squads)
                </summary>
                <ul className="mt-1 list-disc space-y-1 pl-5">
                  {groups.exceptional.map((x, i) => (
                    <li key={i}>{x}</li>
                  ))}
                </ul>
              </details>
            )}
          </>
        ) : groups && singleGroup ? (
          <p className="self-end text-xs text-slate-500 sm:col-span-2" data-testid="membership-single-group">
            Single-group league on this title: its own group id ({String(groups.league_group)}) fills all three fields.
            {targetUnknown && " That id has no known name on this title; it is the value the league's teams already carry."}
          </p>
        ) : o.model === "none" ? null : (
          <>
            <label className="text-xs text-slate-400">
              Conference (number, optional)
              <input
                aria-label="conference number"
                className="mt-1 w-full rounded-md border border-slate-700 bg-slate-950 px-2 py-1 font-mono text-sm text-slate-100"
                value={conference}
                onChange={(e) => setConference(e.target.value)}
                placeholder={cur.conference ? String(cur.conference.value) : ""}
              />
            </label>
            <label className="text-xs text-slate-400">
              Division (number, optional)
              <input
                aria-label="division number"
                className="mt-1 w-full rounded-md border border-slate-700 bg-slate-950 px-2 py-1 font-mono text-sm text-slate-100"
                value={division}
                onChange={(e) => setDivision(e.target.value)}
                placeholder={cur.division ? String(cur.division.value) : ""}
              />
            </label>
          </>
        )}
      </div>
      {groups && !singleGroup && (targetUnknown || targetInferred) && (
        <p className="mt-2 text-xs text-amber-300" data-testid="membership-target-evidence">
          {targetUnknown
            ? `The league's own group id (${String(groups.league_group)}) has no known name on this title.`
            : `The league's group ids on this title are inferred from database usage, not read from the executable in id order.`}
        </p>
      )}
      <div className="mt-2 flex items-center gap-2">
        <Button onClick={() => move.mutate()} disabled={!canMove}>
          {move.isPending ? "Moving…" : "Move team"}
        </Button>
      </div>
      <ul className="mt-2 list-disc space-y-1 pl-5 text-xs text-slate-500" data-testid="membership-notes">
        {o.notes.map((n, i) => (
          <li key={i}>{n}</li>
        ))}
      </ul>
      {move.isError && <p className="mt-2 text-sm text-rose-400">{String(move.error)}</p>}
      {summaryError && (
        <p className="mt-2 text-xs text-amber-300" data-testid="membership-summary-error">
          The move was applied, but refreshing the project summary failed: {summaryError}
        </p>
      )}
      {result && (
        <p className="mt-2 text-xs text-emerald-300" data-testid="membership-result">
          Moved: {result.before.league_label}
          {result.before.division ? ` / ${result.before.division.label}` : ""} → {result.after.league_label}
          {result.after.conference ? ` / ${result.after.conference.label}` : ""}
          {result.after.division ? ` / ${result.after.division.label}` : ""} ({result.writes.length} cell
          {result.writes.length === 1 ? "" : "s"}, one undo step).
        </p>
      )}
    </Card>
  );
}

/// The text of a choosable group: its label, plus the mapping's caveat when it is not traced.
function choiceText(c: { label: string; evidence: string | null }): string {
  if (c.evidence === "inferred") return `${c.label} (inferred)`;
  if (c.evidence === "unknown") return `${c.label} (name unknown)`;
  return c.label;
}

function ChoiceEvidence({ evidence }: { evidence: string | null }) {
  if (evidence !== "inferred" && evidence !== "unknown") return null;
  return (
    <>
      {" "}
      <Pill tone={evidence === "unknown" ? "error" : "warn"}>{evidence === "unknown" ? "name unknown" : "inferred"}</Pill>
    </>
  );
}
