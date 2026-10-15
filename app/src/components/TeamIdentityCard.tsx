import { useEffect, useRef, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import type { ArtIdSwapDto } from "../bindings/ArtIdSwapDto";
import type { TeamSummaryDto } from "../bindings/TeamSummaryDto";
import { Button, Card, Pill } from "./primitives";

/// Phase 2.1 (21 Sep 2026) — a team's two identities under the re-identification contract
/// (research 2026-09-21 handoff 10): the **index** (refused to change; its consumers are counted
/// per table) and the **art id** (never a bare cell edit — the only supported change is a swap
/// with another existing team, one transaction, one undo step, the jersey / award rows of both
/// following).
///
/// Every swap carries an immutable **dispatch context** — the project, the team, the chosen
/// partner and the card generation it was started from (coordinator review R1 F2, 21 Sep 2026).
/// Success, failure, the owning project's summary refresh and its failure all decide by that
/// context, never by the values a later render happens to hold: a completion landing after the
/// card moved to another team or project shows nothing there, while the owning project's
/// summary still refreshes; a completion after unmount touches nothing.
type SwapContext = { projectId: number; teamId: number; other: number; gen: number };

export function TeamIdentityCard({ projectId, teamId, teams }: { projectId: number; teamId: number; teams: TeamSummaryDto[] }) {
  const qc = useQueryClient();
  const { update } = useProject();
  const deps = useQuery({
    queryKey: ["team-identity-deps", projectId, teamId],
    queryFn: () => call("team_identity_dependencies", { projectId, teamId }),
  });
  const [other, setOther] = useState<string>("");
  const [result, setResult] = useState<ArtIdSwapDto | null>(null);
  const [error, setError] = useState<string | null>(null);
  // The card's generation: bumped (and the card reset) whenever the project or team changes, so a
  // request dispatched from an earlier generation — even for the same project and team, after
  // switching away and back — no longer owns this card. `genRef` mirrors it for the callbacks.
  const [gen, setGen] = useState(0);
  const genRef = useRef(0);
  const ctxRef = useRef({ projectId, teamId });
  const mountedRef = useRef(true);
  useEffect(() => {
    ctxRef.current = { projectId, teamId };
    genRef.current += 1;
    setGen(genRef.current);
    setOther("");
    setResult(null);
    setError(null);
  }, [projectId, teamId]);
  useEffect(() => {
    mountedRef.current = true;
    return () => void (mountedRef.current = false);
  }, []);
  /// Does a request from `ctx` still own this card (same generation, still mounted)?
  const ownsCard = (ctx: SwapContext) => mountedRef.current && ctx.gen === genRef.current;
  /// Is the card currently showing the project the request was issued for (for the summary refresh)?
  const ownsProject = (ctx: SwapContext) => mountedRef.current && ctxRef.current.projectId === ctx.projectId;
  const swap = useMutation({
    mutationFn: (ctx: SwapContext) => call("team_artid_swap", { projectId: ctx.projectId, teamA: ctx.teamId, teamB: ctx.other }),
    onSuccess: async (r, ctx) => {
      void qc.invalidateQueries();
      if (ownsCard(ctx)) {
        setResult(r);
        setError(null);
        setOther("");
      }
      let summary;
      try {
        summary = await call("project_summary", { projectId: ctx.projectId });
      } catch (e) {
        if (ownsCard(ctx)) setError(String(e));
        return;
      }
      if (ownsProject(ctx)) update(summary);
    },
    onError: (e, ctx) => {
      if (ownsCard(ctx)) setError(String(e));
    },
  });
  // "Pending" belongs to the card the request was dispatched from, not to whichever card is shown
  // while it is in flight.
  const pending = swap.isPending && swap.variables?.gen === gen;
  if (deps.isLoading) return null;
  if (deps.isError) {
    return (
      <Card title="Identity (index / art id)">
        <p className="text-xs text-rose-300">{String(deps.error)}</p>
      </Card>
    );
  }
  const d = deps.data!;
  const indexRows = d.index_dependencies.reduce((n, x) => n + x.rows, 0);
  const artRows = d.artid_dependencies.reduce((n, x) => n + x.rows, 0);
  const others = teams.filter((t) => Number(t.team_id) !== teamId);
  return (
    <Card
      title="Identity (index / art id)"
      subtitle={`Team index ${String(d.team_id)} · art id ${d.artid == null ? "—" : String(d.artid)}. Changing an identity is not a cell edit: the contract below says what is offered.`}
    >
      <div className="grid gap-4 text-xs sm:grid-cols-2">
        <section data-testid="identity-index">
          <h3 className="font-semibold text-slate-100">
            Index <Pill tone="warn">refused</Pill>
          </h3>
          <p className="text-slate-400">{d.index_change}</p>
          <ul className="mt-1 text-slate-300">
            {d.index_dependencies.map((x) => (
              <li key={`${x.table}.${x.field}`} className="font-mono">
                {x.table}.{x.field}: {x.rows} row{x.rows === 1 ? "" : "s"}
              </li>
            ))}
          </ul>
          <p className="mt-1 text-slate-500">{indexRows} rows in total point at this index.</p>
        </section>
        <section data-testid="identity-artid">
          <h3 className="font-semibold text-slate-100">
            Art id <Pill tone="ok">swap only</Pill>
          </h3>
          <p className="text-slate-400">{d.artid_change}</p>
          <ul className="mt-1 text-slate-300">
            {d.artid_dependencies.map((x) => (
              <li key={`${x.table}.${x.field}`} className="font-mono">
                {x.table}.{x.field}: {x.rows} row{x.rows === 1 ? "" : "s"}
              </li>
            ))}
          </ul>
          <p className="mt-1 text-slate-500">{artRows} rows carry this art id in the database; the art files keyed by it are listed under Team logos.</p>
          {d.teams_sharing_artid.length > 0 && (
            <p className="mt-1 text-amber-300" data-testid="identity-shared">
              Shared with {d.teams_sharing_artid.join(", ")} — a swap involving this art id is refused.
            </p>
          )}
          <div className="mt-2 flex flex-wrap items-center gap-2">
            <label className="text-slate-300">
              swap art ids with{" "}
              <select
                data-testid="identity-swap-other"
                className="rounded border border-slate-700 bg-slate-900 px-1 py-0.5 text-slate-100"
                value={other}
                onChange={(e) => setOther(e.target.value)}
                disabled={pending}
              >
                <option value="">choose a team…</option>
                {others.map((t) => (
                  <option key={String(t.team_id)} value={String(t.team_id)}>
                    {t.name}
                  </option>
                ))}
              </select>
            </label>
            <Button disabled={other === "" || pending} onClick={() => swap.mutate({ projectId, teamId, other: Number(other), gen })} data-testid="identity-swap">
              Swap
            </Button>
          </div>
          {error && (
            <p className="mt-1 text-rose-300" data-testid="identity-swap-error">
              {error}
            </p>
          )}
          {result && (
            <p className="mt-1 text-emerald-300" data-testid="identity-swap-result">
              {result.summary} — {result.outcome.edits} cells written as one step (undo restores both teams).
            </p>
          )}
        </section>
      </div>
    </Card>
  );
}
