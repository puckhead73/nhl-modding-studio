import { useEffect, useRef, useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import type { SettingRowDto } from "../bindings/SettingRowDto";
import type { SettingsEditPlanDto } from "../bindings/SettingsEditPlanDto";
import { Button, Pill } from "./primitives";

/// D3 — edit one stored catalogue default of an eligible setting (NHL 15 PS4): pick the cell
/// (the global default or one screen that stores one), pick the new code, Preview, then Apply the
/// held review. The server holds the exact plan under a ticket bound to this project instance
/// and its revision; Apply sends the ticket only, never the inputs. Refusals (a screen without a
/// stored default, a code outside the row, an excluded row, a stale ticket) are shown as the
/// server states them. Nothing here claims what the game offers or applies.
export function StoredDefaultEditor({ projectId, setting, installRoot = null }: { projectId: number; setting: SettingRowDto; installRoot?: string | null }) {
  const qc = useQueryClient();
  const { update } = useProject();
  const [target, setTarget] = useState<string>("global");
  const [code, setCode] = useState<string>("");
  const [plan, setPlan] = useState<SettingsEditPlanDto | null>(null);
  const [refusal, setRefusal] = useState<string | null>(null);
  const [applied, setApplied] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const genRef = useRef(0);
  const mountedRef = useRef(true);
  // The context shown *now*: read after every await so a completion for an earlier project or row
  // can neither show a result nor push that project's summary into the global state.
  const ctxRef = useRef({ projectId, legacyId: setting.legacy_id });
  ctxRef.current = { projectId, legacyId: setting.legacy_id };
  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      genRef.current += 1;
    };
  }, []);
  useEffect(() => {
    genRef.current += 1;
    setTarget("global");
    setCode("");
    setPlan(null);
    setRefusal(null);
    setApplied(null);
    setError(null);
  }, [projectId, setting.legacy_id]);

  const contextOf = (t: string): bigint | null => (t === "global" ? null : BigInt(t));
  const currentCode = (): bigint | null => {
    if (target === "global") return setting.global?.code ?? null;
    return setting.context_defaults.find((d) => String(d.context) === target)?.stored.code ?? null;
  };
  const same = (gen: number, forProject: number, forRow: bigint) => mountedRef.current && gen === genRef.current && ctxRef.current.projectId === forProject && ctxRef.current.legacyId === forRow;

  const preview = useMutation({
    mutationFn: async () => {
      const gen = genRef.current;
      const forProject = projectId;
      const forRow = setting.legacy_id;
      const r = await call("settings_default_preview", { projectId: forProject, legacyId: forRow, context: contextOf(target), code: BigInt(code), installRoot });
      return { r, gen, forProject, forRow };
    },
    onSuccess: ({ r, gen, forProject, forRow }) => {
      if (!same(gen, forProject, forRow)) return;
      setApplied(null);
      setError(null);
      if (r.plan) {
        setPlan(r.plan);
        setRefusal(null);
      } else {
        setPlan(null);
        setRefusal(`${r.refusal ?? "refused"}${r.refusal_kind ? ` (${r.refusal_kind})` : ""}`);
      }
    },
    onError: (e, _v, _c) => {
      if (mountedRef.current) setError(String(e));
    },
  });

  const apply = useMutation({
    mutationFn: async (ticket: string) => {
      const gen = genRef.current;
      const forProject = projectId;
      const forRow = setting.legacy_id;
      const r = await call("settings_apply_review", { projectId: forProject, ticket });
      return { r, gen, forProject, forRow };
    },
    onSuccess: ({ r, gen, forProject, forRow }) => {
      void qc.invalidateQueries();
      // The summary is that project's: adopt it only while that project is still the one open.
      if (mountedRef.current && ctxRef.current.projectId === forProject) update(r.summary);
      if (!same(gen, forProject, forRow)) return;
      setPlan(null);
      setApplied(r.plan.no_op ? "Nothing to change." : r.plan.kind === "csv" ? `Pending (written by the next copied-target save): ${r.plan.description}` : `Applied: ${r.plan.description}`);
      setError(null);
    },
    onError: (e) => {
      if (mountedRef.current) {
        setError(String(e));
        // A refused apply leaves nothing held that this panel can reuse: preview again.
        setPlan(null);
      }
    },
  });

  const cur = currentCode();
  const choices = setting.choices;
  const canPreview = code.trim() !== "" && !preview.isPending && !apply.isPending;
  return (
    <section data-testid="stored-default-editor" className="rounded-md border border-slate-800 bg-slate-950/40 p-2">
      <div className="mb-1 text-[11px] uppercase tracking-wide text-slate-500">Edit a stored default</div>
      <div className="flex flex-wrap items-center gap-2 text-xs">
        <label className="text-slate-400">
          cell{" "}
          <select
            data-testid="target-select"
            value={target}
            onChange={(e) => {
              setTarget(e.target.value);
              setPlan(null);
              setRefusal(null);
            }}
            className="rounded border border-slate-700 bg-slate-950 px-1 py-0.5 text-xs text-slate-100"
          >
            <option value="global">global default</option>
            {setting.context_defaults.map((d) => (
              <option key={String(d.context)} value={String(d.context)}>
                screen {String(d.context)}
                {d.context_name ? ` ${d.context_name}` : ""}
              </option>
            ))}
          </select>
        </label>
        <span className="text-slate-500">
          now <span className="font-mono text-slate-300">{cur === null ? "—" : String(cur)}</span>
        </span>
        <label className="text-slate-400">
          new code{" "}
          {choices.length > 0 ? (
            <select
              data-testid="code-select"
              value={code}
              onChange={(e) => {
                setCode(e.target.value);
                setPlan(null);
                setRefusal(null);
              }}
              className="rounded border border-slate-700 bg-slate-950 px-1 py-0.5 text-xs text-slate-100"
            >
              <option value="">choose…</option>
              {choices.map((c) => (
                <option key={String(c.code)} value={String(c.code)}>
                  {String(c.code)} — {c.text ?? c.key}
                </option>
              ))}
            </select>
          ) : (
            <input
              data-testid="code-input"
              value={code}
              onChange={(e) => {
                setCode(e.target.value.replace(/[^0-9-]/g, ""));
                setPlan(null);
                setRefusal(null);
              }}
              placeholder={setting.max !== null ? `0..${setting.max}` : "code"}
              className="w-16 rounded border border-slate-700 bg-slate-950 px-1 py-0.5 font-mono text-xs text-slate-100"
            />
          )}
        </label>
        <Button variant="secondary" className="px-2 py-0.5 text-xs" disabled={!canPreview} onClick={() => preview.mutate()} data-testid="preview-button">
          Preview
        </Button>
        {setting.label_source !== "explicit" && choices.length > 0 && <Pill tone="warn">labels {setting.label_source}</Pill>}
      </div>
      {refusal && (
        <div className="mt-2 text-amber-300/90" data-testid="edit-refusal">
          {refusal}
        </div>
      )}
      {error && (
        <div className="mt-2 text-rose-300" data-testid="edit-error">
          {error}
        </div>
      )}
      {plan && (
        <div className="mt-2 space-y-1 text-xs" data-testid="edit-plan">
          <div className="text-slate-200">{plan.description}</div>
          <div className="font-mono text-[10px] text-slate-500">
            {plan.kind === "csv" ? `${plan.table_name} line ${plan.record}` : `${plan.table_name} record ${plan.record}`} · {String(plan.before)} → {String(plan.after)} · {plan.kind === "csv" ? `archive ${plan.archive ?? ""}` : `copy ${plan.copy}`}
          </div>
          {plan.kind === "csv" && (
            <ul className="font-mono text-[10px] text-slate-500" data-testid="edit-plan-members">
              {plan.members.map((m) => (
                <li key={m.member}>
                  {m.member} ({m.stored_form}): line {m.cells.map((c) => `${c[0]} ${c[1]} ${c[2]}→${c[3]}`).join("; ")} · {String(m.source_stored_len ?? "?")} → {String(m.new_stored_len)} B stored{m.placement_prediction ? ` · ${m.placement_prediction}` : ""}
                </li>
              ))}
            </ul>
          )}
          {plan.paired && <Pill tone="info">paired: two cells in two members, one operation</Pill>}
          <ul className="list-disc pl-4 text-slate-500">
            {plan.notes.map((n) => (
              <li key={n}>{n}</li>
            ))}
            {plan.stock_anomalies.length > 0 && <li>{plan.stock_anomalies.length} stock anomalies preserved (see the disclosures).</li>}
          </ul>
          <div className="flex items-center gap-2">
            <Button className="px-2 py-0.5 text-xs" disabled={apply.isPending} onClick={() => apply.mutate(plan.ticket)} data-testid="apply-button">
              {plan.no_op ? "Nothing to apply" : "Apply"}
            </Button>
            <Button variant="ghost" className="px-2 py-0.5 text-xs" onClick={() => setPlan(null)} data-testid="discard-button">
              Discard
            </Button>
          </div>
        </div>
      )}
      {applied && (
        <div className="mt-2 text-emerald-300/90" data-testid="edit-applied">
          {applied}
        </div>
      )}
    </section>
  );
}
