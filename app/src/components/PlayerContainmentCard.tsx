// Phase 3 (22 Sep 2026) — the read-only "Portrait & head containment" card.
//
// Summary mode (Player assets page): the title's contract, how many players are bespoke / pool,
// how many of those have their files, the `hasportrait` × file cross-tab shown as data (the flag
// is never derived from a file nor written from one), the shipped-but-unreferenced pools, and the
// names the research calls "shipped without asset". Player mode (asset detail): that player's ids,
// flag, file evidence and note.
//
// Result ownership: keyed on project + install root + platform; the query is shared by both modes.
//
// Receipt 38 (research R124): player mode also offers **Reassign portrait / head** to another asset
// the attached tree ships — Check shows the plan (the files that make the target valid, or why it is
// refused, and the disclosures); Apply stages one ordinary field edit (Pending review, undo / redo,
// Save). A plan is re-derived on Apply; a stale or refused plan writes nothing.
//
// Receipt 66: on PS4 a third row reassigns the hair style (NHL 17–21, checked against the title's own vault; the backend
// refuses other titles by name). A PS4 plan lists the exact source hashes it checked behind a disclosure; Apply re-reads them.

import { useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { call } from "../lib/client";
import type { AssetRootsArgsDto } from "../bindings/AssetRootsArgsDto";
import type { PlayerContainmentViewDto } from "../bindings/PlayerContainmentViewDto";
import type { PlayerReassignPlanDto } from "../bindings/PlayerReassignPlanDto";
import { Button, Card, Spinner } from "./primitives";

const STATUS_TONE: Record<string, string> = {
  present: "border-emerald-700/60 bg-emerald-900/30 text-emerald-200",
  missing: "border-rose-700/60 bg-rose-900/30 text-rose-200",
  unknown: "border-amber-700/60 bg-amber-900/30 text-amber-200",
};

function Badge({ tone, children, testId }: { tone: string; children: React.ReactNode; testId?: string }) {
  return (
    <span className={"rounded border px-1.5 py-0.5 text-[10px] font-semibold uppercase tracking-wide " + tone} data-testid={testId}>
      {children}
    </span>
  );
}

export function usePlayerContainment(projectId: number, rootArgs: AssetRootsArgsDto, installDir: string | null) {
  return useQuery({
    queryKey: ["playerContainment", projectId, installDir ?? "", rootArgs.platform ?? "", rootArgs.rendering_override ?? "", rootArgs.fe_override ?? ""],
    queryFn: () => call("player_containment", { projectId, roots: rootArgs }),
    staleTime: Infinity,
    retry: false,
  });
}

export function PlayerContainmentCard({
  projectId,
  rootArgs,
  installDir,
  bioRecord,
}: {
  projectId: number;
  rootArgs: AssetRootsArgsDto;
  installDir: string | null;
  /** When given, the card shows this player's row instead of the title summary. */
  bioRecord?: number;
}) {
  const q = usePlayerContainment(projectId, rootArgs, installDir);
  if (q.isError) {
    return (
      <Card title="Portrait & head containment">
        <p className="text-xs text-rose-300" data-testid="containment-error">
          {String(q.error)}
        </p>
      </Card>
    );
  }
  const d: PlayerContainmentViewDto | undefined = q.data;
  if (!d) return <Spinner label="Reading the portrait and head containment…" />;

  if (bioRecord != null) {
    const p = d.players.find((x) => x.bio_record === bioRecord);
    if (!p) {
      return (
        <p className="text-xs text-slate-500" data-testid="containment-player-none">
          This record is not in the containment view.
        </p>
      );
    }
    return (
      <div className="space-y-1 text-xs" data-testid="containment-player" data-bio-record={p.bio_record}>
        <p className="flex flex-wrap items-center gap-2 text-slate-300">
          <span>
            art id {p.portrait_id} · head id {p.head_id} ·{" "}
            <span data-testid="containment-player-class">{p.class === "bespoke" ? "own head (head id = art id)" : p.class === "pool" ? "pool head" : "no measured class"}</span>
          </span>
          <span>
            portrait <Badge tone={STATUS_TONE[p.portrait] ?? ""} testId="containment-player-portrait">{p.portrait}</Badge>
          </span>
          <span>
            head <Badge tone={STATUS_TONE[p.head] ?? ""} testId="containment-player-head">{p.head}</Badge>
          </span>
          <span data-testid="containment-player-flag">
            {p.has_portrait_flag == null ? "no hasportrait column" : `hasportrait = ${p.has_portrait_flag ? 1 : 0} (as stored)`}
          </span>
        </p>
        {p.note ? (
          <p className="text-amber-300/90" data-testid="containment-player-note">
            {p.note}
          </p>
        ) : null}
        <ReassignRow projectId={projectId} rootArgs={rootArgs} bioRecord={p.bio_record} kind="portrait" current={Number(p.portrait_id)} />
        <ReassignRow projectId={projectId} rootArgs={rootArgs} bioRecord={p.bio_record} kind="head" current={Number(p.head_id)} />
        {d.summary.contract_status === "ps4" ? <ReassignRow projectId={projectId} rootArgs={rootArgs} bioRecord={p.bio_record} kind="hair" /> : null}
      </div>
    );
  }

  const s = d.summary;
  return (
    <Card
      title="Portrait & head containment"
      subtitle="What the attached install ships for the roster's art ids and head ids, and the hasportrait flag as stored. Read-only: the flag is never derived from a file and never written from one."
    >
      <div className="space-y-2 text-xs" data-testid="containment-summary">
        <p className="flex flex-wrap items-center gap-2">
          <Badge tone={s.contract_status === "unmeasured" ? STATUS_TONE.unknown : "border-sky-700/60 bg-sky-900/30 text-sky-200"} testId="containment-contract-status">
            {s.contract_status === "gen7" ? "gen7 file-name rules" : s.contract_status === "ps4" ? "PS4 portrait rule · heads not measured" : "not measured"}
          </Badge>
          <span className="text-slate-500" title={s.contract}>
            {s.players} players
          </span>
        </p>
        {s.contract_status !== "unmeasured" ? (
          <>
            <table className="w-full text-left" data-testid="containment-table">
              <tbody>
                <tr>
                  <td className="pr-3 text-slate-400">Own heads (head id = art id)</td>
                  <td className="text-slate-200" data-testid="containment-bespoke">
                    {s.bespoke} — head file {s.bespoke_head_present}, portrait file {s.bespoke_portrait_present}
                  </td>
                </tr>
                <tr>
                  <td className="pr-3 text-slate-400">Pool heads (floor {s.pool_floor ?? "—"})</td>
                  <td className="text-slate-200" data-testid="containment-pool">
                    {s.pool_players} — head file {s.pool_head_present}
                  </td>
                </tr>
                {s.has_flag_column ? (
                  <tr>
                    <td className="pr-3 text-slate-400">hasportrait × portrait file</td>
                    <td className="text-slate-200" data-testid="containment-flag">
                      flagged with file {s.flag1_file1} · flagged without {s.flag1_file0} · clear with file {s.flag0_file1} · clear without {s.flag0_file0}
                    </td>
                  </tr>
                ) : null}
                <tr>
                  <td className="pr-3 text-slate-400">Shipped, referenced by no row</td>
                  <td className="text-slate-200" data-testid="containment-unused">
                    portraits {s.unused_portraits ?? "unknown"} of {s.shipped_portraits ?? "?"} · heads {s.unused_heads ?? "unknown"} of {s.shipped_heads ?? "?"}
                  </td>
                </tr>
              </tbody>
            </table>
            {s.bespoke_missing_head.length + s.bespoke_missing_portrait.length + s.flagged_without_file.length > 0 ? (
              <div className="space-y-0.5" data-testid="containment-residuals">
                {s.bespoke_missing_head.length > 0 ? <p className="text-amber-300/90">Own head, no head file (shipped without asset): {s.bespoke_missing_head.join(", ")}</p> : null}
                {s.bespoke_missing_portrait.length > 0 ? <p className="text-amber-300/90">Own head, no portrait file: {s.bespoke_missing_portrait.join(", ")}</p> : null}
                {s.flagged_without_file.length > 0 ? <p className="text-amber-300/90">Flagged as having a portrait, no file: {s.flagged_without_file.join(", ")}</p> : null}
              </div>
            ) : null}
          </>
        ) : null}
        <ul className="list-disc space-y-0.5 pl-4 text-slate-500" data-testid="containment-notes">
          {s.notes.map((n, i) => (
            <li key={i}>{n}</li>
          ))}
        </ul>
      </div>
    </Card>
  );
}

const KIND_LABEL: Record<"portrait" | "head" | "hair", string> = { portrait: "portrait", head: "head", hair: "hair style" };

/** Reassign one of the player's asset ids to another shipped asset (receipt 38; PS4 NHL 17–21 hair, receipt 66). */
function ReassignRow({
  projectId,
  rootArgs,
  bioRecord,
  kind,
  current,
}: {
  projectId: number;
  rootArgs: AssetRootsArgsDto;
  bioRecord: number;
  kind: "portrait" | "head" | "hair";
  /** The stored id, when this view carries it (the hair id comes back with the plan). */
  current?: number;
}) {
  const qc = useQueryClient();
  const [target, setTarget] = useState("");
  const [plan, setPlan] = useState<PlayerReassignPlanDto | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [done, setDone] = useState<string | null>(null);
  const id = Number(target);
  const valid = target.trim() !== "" && Number.isInteger(id) && id >= 0;
  const run = async (apply: boolean) => {
    if (!valid) return;
    setBusy(true);
    setError(null);
    setDone(null);
    try {
      // Receipt 63: an apply names the plan it reviewed; the backend refuses when the row or files changed since.
      const reviewed = apply ? (plan && plan.target_id === id ? plan.fingerprint : null) : null;
      const r = await call("player_reassign", { projectId, roots: rootArgs, bioRecord, kind, target: id, apply, reviewed });
      setPlan(r);
      if (apply && r.applied) {
        setDone(`${kind === "portrait" ? "Portrait" : kind === "head" ? "Head" : "Hair style"} set to ${id} — staged as one edit; review it under Pending changes, undo it, or Save.`);
        setPlan(null);
        setTarget("");
        void qc.invalidateQueries();
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };
  const shown = plan && plan.target_id === id ? plan : null;
  return (
    <div className="mt-1 rounded border border-slate-800 p-1.5" data-testid={`reassign-${kind}`}>
      <div className="flex flex-wrap items-center gap-2">
        <span className="text-slate-400">
          Reassign {KIND_LABEL[kind]} ({current ?? shown?.current_id ?? "as stored"}) to
        </span>
        <input
          className="w-24 rounded border border-slate-700 bg-slate-950 px-1 py-0.5 font-mono"
          value={target}
          inputMode="numeric"
          disabled={busy}
          onChange={(e) => {
            setTarget(e.target.value);
            setPlan(null);
            setError(null);
            setDone(null);
          }}
          aria-label={`new ${kind} id`}
          data-testid={`reassign-${kind}-target`}
        />
        <Button variant="secondary" className="px-2 py-0.5 text-xs" disabled={!valid || busy} onClick={() => void run(false)} data-testid={`reassign-${kind}-check`}>
          Check
        </Button>
        <Button className="px-2 py-0.5 text-xs" disabled={!shown?.allowed || busy} onClick={() => void run(true)} data-testid={`reassign-${kind}-apply`}>
          Apply
        </Button>
      </div>
      {shown && (
        <div className="mt-1 space-y-0.5" data-testid={`reassign-${kind}-plan`}>
          {shown.refusals.map((r) => (
            <p key={r} className="text-rose-300" data-testid={`reassign-${kind}-refusal`}>
              {r}
            </p>
          ))}
          {shown.evidence.map((f) => (
            <p key={f} className="font-mono text-[10px] text-emerald-300/80">
              ships: {f}
            </p>
          ))}
          {shown.notes.map((n) => (
            <p key={n} className="text-slate-500">
              {n}
            </p>
          ))}
          {shown.source_identity.length > 0 ? (
            <details data-testid={`reassign-${kind}-source`}>
              <summary className="cursor-pointer text-slate-500">Checked source (exact hashes, re-read on Apply)</summary>
              {shown.source_identity.map((l) => (
                <p key={l} className="break-all font-mono text-[10px] text-slate-500">
                  {l}
                </p>
              ))}
            </details>
          ) : null}
        </div>
      )}
      {error && (
        <p className="mt-1 text-rose-300" data-testid={`reassign-${kind}-error`}>
          {error}
        </p>
      )}
      {done && (
        <p className="mt-1 text-emerald-300" data-testid={`reassign-${kind}-done`}>
          {done}
        </p>
      )}
    </div>
  );
}
