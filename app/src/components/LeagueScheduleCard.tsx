import { scheduleName, flexibleSchedule } from "./TeamScheduleCalendar";
import { queueFieldWrite } from "../lib/fieldWrites";
import { useEffect, useMemo, useRef, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";
import { call } from "../lib/client";
import type { ScheduleGameEditArgs, ScheduleGenerationConfigArgs, ScheduleTeamMappingArgs } from "../lib/client";
import { useProject } from "../lib/project";
import type { ScheduleViewDto } from "../bindings/ScheduleViewDto";
import type { ScheduledGameDto } from "../bindings/ScheduledGameDto";
import type { ScheduleChangePlanDto } from "../bindings/ScheduleChangePlanDto";
import type { ScheduleImportPlanDto } from "../bindings/ScheduleImportPlanDto";
import type { ScheduleGenerationOptionsDto } from "../bindings/ScheduleGenerationOptionsDto";
import type { ScheduleGenerationPlanDto } from "../bindings/ScheduleGenerationPlanDto";
import type { ScheduleApplyResultDto } from "../bindings/ScheduleApplyResultDto";
import { Button, Card, Pill, Spinner } from "./primitives";

/// Phase 1.2 / P1-A — the league's source schedule tables as the database ships them, with the
/// P1-A workflow on top: edit a game (home / away / date) with a validated preview, export the
/// table as CSV or JSON, import a file through an explicit team mapping, and generate a balanced
/// round-robin where the table's row count allows it (else the exact blocker). Slots are league
/// ranks resolved to teams by the server; the card never computes a slot.
///
/// Request ownership (R1 F2 / R2): every preview is held server-side under an opaque `ticket`
/// bound to the issuing project instance and its revision, and Apply sends only that ticket —
/// never the inputs. In the card, every async
/// completion (preview, apply, dialog, summary refresh) is bound to the context it started for
/// (`projectId` / `leagueCode` / table) and, for previews, to the input generation at the time
/// of the request; anything that comes back for an older context or generation is dropped.
/// Changing an input invalidates the shown plan immediately; controls lock while an apply runs.
export function LeagueScheduleCard({ projectId, leagueCode }: { projectId: number; leagueCode: number }) {
  const qc = useQueryClient();
  const { update } = useProject();
  const tables = useQuery({
    queryKey: ["schedule-tables", projectId],
    queryFn: () => call("schedule_tables", { projectId }),
  });
  // A league's own tables, plus the tournament templates shown with it (R115; display only — they
  // carry no slot map, so they stay browse-only).
  const mine = (tables.data ?? []).filter(
    (t) => (t.league_code != null && Number(t.league_code) === leagueCode) || (t.shown_with_league != null && Number(t.shown_with_league) === leagueCode),
  );
  const [picked, setPicked] = useState<{ projectId: number; leagueCode: number; table: string } | null>(null);
  // A pick belongs to the project/league it was made in; otherwise the league's source table.
  const pickedHere = picked && picked.projectId === projectId && picked.leagueCode === leagueCode && mine.some((t) => t.studio_table === picked.table) ? picked.table : null;
  const table = pickedHere ?? mine.find((t)=>t.name==="nhlschedulelongnoleap")?.studio_table ?? mine.find((t) => t.role === "source")?.studio_table ?? mine[0]?.studio_table ?? null;
  const ctxKey = `${projectId}/${leagueCode}/${table ?? ""}`;
  const view = useQuery({
    queryKey: ["schedule-view", projectId, table],
    queryFn: () => call("schedule_view", { projectId, studioTable: table! }),
    enabled: table != null,
  });
  const [page, setPage] = useState(0);
  const [editing, setEditing] = useState<number | null>(null);
  const [panel, setPanel] = useState<"none" | "import" | "generate">("none");
  const [notice, setNotice] = useState<string | null>(null);
  const [exportError, setExportError] = useState<string | null>(null);
  const [exporting, setExporting] = useState(false);
  const PAGE = 60;

  const mountedRef = useRef(true);
  const ctxRef = useRef(ctxKey);
  ctxRef.current = ctxKey;
  const projectRef = useRef(projectId);
  projectRef.current = projectId;
  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);
  // A new project/league/table: nothing held for the old one is shown again.
  useEffect(() => {
    setEditing(null);
    setPanel("none");
    setNotice(null);
    setExportError(null);
    setPage(0);
  }, [ctxKey]);

  /** After an apply for `startedCtx`: refresh queries; refresh the project summary only if the
   *  same project is still open when it comes back; show the notice only in the same context. */
  async function afterApply(startedCtx: string, startedProject: number, message: string) {
    void qc.invalidateQueries();
    const summary = await call("project_summary", { projectId: startedProject });
    if (!mountedRef.current) return;
    if (projectRef.current === startedProject) update(summary);
    if (ctxRef.current === startedCtx) setNotice(message);
  }

  async function exportAs(format: "csv" | "json") {
    const started = ctxRef.current;
    const forProject = projectId;
    const forTable = table!;
    const name = v?.table.name ?? "schedule";
    setExporting(true);
    setExportError(null);
    try {
      const outPath = await saveDialog({
        title: `Export ${name} as ${format.toUpperCase()}`,
        defaultPath: `${name}.${format}`,
        filters: [{ name: format.toUpperCase(), extensions: [format] }],
      });
      // The dialog came back for a context that is gone: nothing is exported for it.
      if (!outPath || !mountedRef.current || ctxRef.current !== started) return;
      const r = await call("schedule_export", { projectId: forProject, studioTable: forTable, format, outPath });
      if (mountedRef.current && ctxRef.current === started) setNotice(`Exported ${r.suggested_filename} (${r.format.toUpperCase()}).`);
    } catch (e) {
      if (mountedRef.current && ctxRef.current === started) setExportError(String(e));
    } finally {
      if (mountedRef.current) setExporting(false);
    }
  }

  if (tables.isLoading) return null;
  if (mine.length === 0) return null;
  const v = view.data ?? null;
  const games = v?.games ?? [];
  const pages = Math.max(1, Math.ceil(games.length / PAGE));
  const shown = games.slice(page * PAGE, page * PAGE + PAGE);
  const editable = !!v && v.slot_map.length > 0;

  return (
    <Card
      title="Source schedule"
      subtitle="Choose a season template. Schedule changes appear in both teams’ calendars."
    >
      <div className="mb-2 flex flex-wrap items-center gap-2 text-xs">
        {mine.map((t) => (
          <button
            key={t.studio_table}
            type="button"
            onClick={() => {
              setPicked({ projectId, leagueCode, table: t.studio_table });
              setPage(0);
            }}
            className={
              "rounded border px-2 py-0.5 " +
              (t.studio_table === table ? "border-sky-500 text-sky-200" : "border-slate-700 text-slate-300 hover:border-slate-500")
            }
            title={t.league_match}
          >
            {scheduleName(t.name)} · {t.rows} games
          </button>
        ))}
      </div>
      {view.isLoading && <Spinner label="Reading the schedule…" />}
      {view.isError && <p className="text-sm text-rose-400">{String(view.error)}</p>}
      {v && (
        <>
          <div className="mb-2 flex flex-wrap items-center gap-2 text-xs text-slate-400" data-testid="schedule-summary">
            <span>
              {v.games.length} games · {v.first_date ?? "?"} → {v.last_date ?? "?"} · {v.slots.length} teams with games
            </span>
            {v.findings.length === 0 ? <Pill tone="ok">consistent</Pill> : <Pill tone="warn">{v.findings.length} finding{v.findings.length === 1 ? "" : "s"}</Pill>}
            <span className="ml-auto flex flex-wrap gap-1">
              <Button variant="secondary" className="px-2 py-0.5 text-xs" disabled={!editable || exporting} onClick={() => void exportAs("csv")}>
                Export CSV
              </Button>
              <Button variant="secondary" className="px-2 py-0.5 text-xs" disabled={!editable || exporting} onClick={() => void exportAs("json")}>
                Export JSON
              </Button>
              <Button variant="secondary" className="px-2 py-0.5 text-xs" disabled={!editable} onClick={() => setPanel(panel === "import" ? "none" : "import")}>
                Import…
              </Button>
              <Button variant="secondary" className="px-2 py-0.5 text-xs" disabled={!editable} onClick={() => setPanel(panel === "generate" ? "none" : "generate")}>
                Generate…
              </Button>
            </span>
          </div>
          {!editable && !v.tournament_note && (
            <p className="mb-2 text-xs text-amber-200/80">
              This table's teams cannot be named ({v.table.league_match}); it is shown read-only. Editing by team, import and generation need a resolved league.
            </p>
          )}
          {v.tournament_note && (
            <p className="mb-2 text-xs text-sky-200/80" data-testid="schedule-tournament-note">
              {v.tournament_note}
            </p>
          )}
          {exportError && <p className="mb-2 text-xs text-rose-400">{exportError}</p>}
          {notice && (
            <p className="mb-2 rounded border border-emerald-800/60 bg-emerald-950/30 px-2 py-1 text-xs text-emerald-200" data-testid="schedule-notice">
              {notice}
            </p>
          )}
          {v.findings.length > 0 && (
            <ul className="mb-2 list-disc space-y-1 pl-5 text-xs text-amber-200/80">
              {v.findings.map((f, i) => (
                <li key={i}>{f}</li>
              ))}
            </ul>
          )}
          {panel === "import" && editable && (
            <ImportPanel
              key={ctxKey}
              ctxKey={ctxKey}
              ctxRef={ctxRef}
              projectId={projectId}
              studioTable={table!}
              view={v}
              onApplied={(msg) => {
                if (mountedRef.current && ctxRef.current === ctxKey) setPanel("none");
                void afterApply(ctxKey, projectId, msg);
              }}
            />
          )}
          {panel === "generate" && editable && (
            <GeneratePanel
              key={ctxKey}
              ctxKey={ctxKey}
              ctxRef={ctxRef}
              projectId={projectId}
              studioTable={table!}
              view={v}
              onApplied={(msg) => {
                if (mountedRef.current && ctxRef.current === ctxKey) setPanel("none");
                void afterApply(ctxKey, projectId, msg);
              }}
            />
          )}
          <details className="mb-2 text-xs text-slate-400">
            <summary className="cursor-pointer">Games per team</summary>
            <div className="mt-1 grid gap-x-4 sm:grid-cols-3">
              {v.slots.map((s) => (
                <div key={String(s.slot)}>
                  <span className="font-mono text-slate-500">{String(s.slot).padStart(2, " ")}</span> {s.team ?? "—"}{" "}
                  <span className="text-slate-500">({s.games})</span>
                </div>
              ))}
              {v.unresolved_slots.map((s) => (
                <div key={"u" + String(s.slot)} className="text-amber-300/80">
                  <span className="font-mono">{String(s.slot).padStart(2, " ")}</span> no team at this position ({s.games})
                </div>
              ))}
            </div>
          </details>
          <div className="overflow-x-auto">
            <table className="w-full text-left text-xs">
              <thead className="text-slate-500">
                <tr>
                  <th className="py-1 pr-2">#</th>
                  <th className="py-1 pr-2">date</th>
                  <th className="py-1 pr-2">home</th>
                  <th className="py-1 pr-2">away</th>
                  <th className="py-1 pr-2">round</th>
                  {v.fields.some(([k]) => k === "status") && <th className="py-1 pr-2">status</th>}
                  {editable && <th className="py-1 pr-2"></th>}
                </tr>
              </thead>
              <tbody>
                {shown.map((g) =>
                  editing === g.record ? (
                    <tr key={g.record} className="border-t border-slate-800/60 bg-slate-900/60">
                      <td colSpan={editable ? 7 : 6} className="py-1 pr-2">
                        <GameEditor
                          key={`${ctxKey}/${g.record}`}
                          ctxKey={ctxKey}
                          ctxRef={ctxRef}
                          projectId={projectId}
                          studioTable={table!}
                          view={v}
                          game={g}
                          onCancel={() => setEditing(null)}
                          onApplied={(msg) => {
                            if (mountedRef.current && ctxRef.current === ctxKey) setEditing(null);
                            void afterApply(ctxKey, projectId, msg);
                          }}
                        />
                      </td>
                    </tr>
                  ) : (
                    <tr key={g.record} className="border-t border-slate-800/60">
                      <td className="py-0.5 pr-2 font-mono text-slate-500">{String(g.index)}</td>
                      <td className="py-0.5 pr-2">{g.date}</td>
                      <td className="py-0.5 pr-2">{g.home ?? <SlotLabel label={g.home_label} slot={g.home_slot} />}</td>
                      <td className="py-0.5 pr-2">{g.away ?? <SlotLabel label={g.away_label} slot={g.away_slot} />}</td>
                      <td className="py-0.5 pr-2 text-slate-500" title={g.round_label ? `round ${String(g.round)}` : undefined}>
                        {g.round_label ?? String(g.round)}
                      </td>
                      {v.fields.some(([k]) => k === "status") && <td className="py-0.5 pr-2 text-slate-500">{g.status == null ? "" : String(g.status)}</td>}
                      {editable && (
                        <td className="py-0.5 pr-2 text-right">
                          <button type="button" className="text-sky-300 hover:underline" onClick={() => setEditing(g.record)} data-testid={`edit-game-${g.record}`}>
                            edit
                          </button>
                        </td>
                      )}
                    </tr>
                  ),
                )}
              </tbody>
            </table>
          </div>
          {pages > 1 && (
            <div className="mt-2 flex items-center gap-2 text-xs text-slate-400">
              <button type="button" className="rounded border border-slate-700 px-2 py-0.5 disabled:opacity-40" disabled={page === 0} onClick={() => setPage(page - 1)}>
                ‹
              </button>
              <span>
                page {page + 1} / {pages}
              </span>
              <button type="button" className="rounded border border-slate-700 px-2 py-0.5 disabled:opacity-40" disabled={page + 1 >= pages} onClick={() => setPage(page + 1)}>
                ›
              </button>
            </div>
          )}
        </>
      )}
    </Card>
  );
}

const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

type Ctx = { ctxKey: string; ctxRef: React.MutableRefObject<string> };

/**
 * Preview/apply lifecycle shared by the three panels. A preview request carries the input
 * generation and context it was made for; its response is kept only if both are still current.
 * Apply sends the held review id and locks the inputs until it completes; a second Apply while
 * one runs is ignored.
 */
function useReview<P>({ ctxKey, ctxRef }: Ctx, planOf: (p: P) => ScheduleChangePlanDto) {
  const [plan, setPlan] = useState<P | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [previewing, setPreviewing] = useState(false);
  const [applying, setApplying] = useState(false);
  const genRef = useRef(0);
  const applyingRef = useRef(false);
  const mountedRef = useRef(true);
  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);
  const live = () => mountedRef.current && ctxRef.current === ctxKey;
  /** Inputs changed: the shown plan no longer describes them; pending previews become stale. */
  function invalidate() {
    genRef.current += 1;
    setPlan(null);
  }
  async function preview(request: () => Promise<P>) {
    if (applyingRef.current) return;
    const gen = genRef.current;
    setPreviewing(true);
    try {
      const p = await request();
      if (!live() || gen !== genRef.current) return; // stale: newer inputs or another context
      setPlan(p);
      setError(null);
    } catch (e) {
      if (live() && gen === genRef.current) setError(String(e));
    } finally {
      if (mountedRef.current) setPreviewing(false);
    }
  }
  async function apply(projectId: number, onApplied: (r: ScheduleApplyResultDto) => void) {
    if (applyingRef.current || !plan) return;
    const held = planOf(plan);
    if (held.validation.errors.length > 0 || held.cells === 0) return;
    applyingRef.current = true;
    setApplying(true);
    try {
      const r = await queueFieldWrite(projectId,()=>call("schedule_apply_review", { projectId, ticket: held.ticket }));
      if (mountedRef.current) setPlan(null);
      // The server applied it: the card refreshes regardless of whether this panel is still
      // shown (the card checks its own ownership before touching the project or the notice).
      onApplied(r);
    } catch (e) {
      if (live()) {
        setError(String(e));
        setPlan(null); // a refused review is gone server-side; preview again
      }
    } finally {
      applyingRef.current = false;
      if (mountedRef.current) setApplying(false);
    }
  }
  return { plan, error, previewing, applying, invalidate, preview, apply, setError };
}

/** Validation errors/warnings + the games-per-team summary of a plan. */
function PlanSummary({ plan, label }: { plan: ScheduleChangePlanDto; label: string }) {
  const val = plan.validation;
  return (
    <div className="mt-2 space-y-1 text-xs" data-testid="schedule-plan" data-review={plan.ticket}>
      <div className="flex flex-wrap items-center gap-2">
        <span className="text-slate-300">
          {label}: {plan.rows_changed} row{plan.rows_changed === 1 ? "" : "s"}, {plan.cells} cell{plan.cells === 1 ? "" : "s"}
        </span>
        {val.errors.length === 0 ? <Pill tone="ok">valid</Pill> : <Pill tone="error">{val.errors.length} refusal{val.errors.length === 1 ? "" : "s"}</Pill>}
        {val.warnings.length > 0 && <Pill tone="warn">{val.warnings.length} warning{val.warnings.length === 1 ? "" : "s"}</Pill>}
        {val.first_date && (
          <span className="text-slate-500">
            {val.first_date} → {val.last_date}
          </span>
        )}
      </div>
      {val.errors.length > 0 && (
        <ul className="list-disc pl-5 text-rose-300">
          {val.errors.map((e, i) => (
            <li key={i}>{e}</li>
          ))}
        </ul>
      )}
      {val.warnings.length > 0 && (
        <ul className="list-disc pl-5 text-amber-200/80">
          {val.warnings.map((w, i) => (
            <li key={i}>{w}</li>
          ))}
        </ul>
      )}
      {plan.notes.length > 0 && (
        <ul className="list-disc pl-5 text-slate-500">
          {plan.notes.map((n, i) => (
            <li key={i}>{n}</li>
          ))}
        </ul>
      )}
      {plan.after.length > 0 && (
        <details className="text-slate-400">
          <summary className="cursor-pointer">Changed rows ({plan.after.length}{plan.rows_changed > plan.after.length ? ` of ${plan.rows_changed}` : ""})</summary>
          <div className="mt-1 max-h-48 overflow-auto font-mono">
            {plan.after.map((a, i) => {
              const b = plan.before[i];
              return (
                <div key={a.record}>
                  #{String(a.index)} {b ? `${b.date} ${b.home ?? "slot " + String(b.home_slot)} vs ${b.away ?? "slot " + String(b.away_slot)}` : ""} → {a.date}{" "}
                  {a.home ?? "slot " + String(a.home_slot)} vs {a.away ?? "slot " + String(a.away_slot)}
                </div>
              );
            })}
          </div>
        </details>
      )}
    </div>
  );
}

/** Inline editor for one game: home / away from the league's slot map, calendar date. */
function GameEditor({
  ctxKey,
  ctxRef,
  projectId,
  studioTable,
  view,
  game,
  onCancel,
  onApplied,
}: Ctx & {
  projectId: number;
  studioTable: string;
  view: ScheduleViewDto;
  game: ScheduledGameDto;
  onCancel: () => void;
  onApplied: (message: string) => void;
}) {
  const homeNow = view.slot_map.find((e) => e.slot === game.home_slot);
  const awayNow = view.slot_map.find((e) => e.slot === game.away_slot);
  const [home, setHome] = useState<string>(homeNow ? String(homeNow.team_id) : "");
  const [away, setAway] = useState<string>(awayNow ? String(awayNow.team_id) : "");
  const [month, setMonth] = useState<string>(String(Number(game.month) + 1));
  const [day, setDay] = useState<string>(String(Number(game.day) + 1));
  const rv = useReview<ScheduleChangePlanDto>({ ctxKey, ctxRef }, (p) => p);
  const edit = (): ScheduleGameEditArgs => ({
    record: game.record,
    home_team_id: home === "" ? null : Number(home),
    away_team_id: away === "" ? null : Number(away),
    month: month.trim() === "" ? null : Number(month) - 1,
    day: day.trim() === "" ? null : Number(day) - 1,
  });
  const set = (f: (v: string) => void) => (v: string) => {
    rv.invalidate();
    f(v);
  };
  const locked = rv.applying;
  const canApply = !!rv.plan && rv.plan.validation.errors.length === 0 && rv.plan.cells > 0 && !locked;
  return (
    <div className="text-xs" data-testid="game-editor">
      <div className="flex flex-wrap items-end gap-2">
        <span className="font-mono text-slate-500">#{String(game.index)}</span>
        <label className="flex flex-col">
          <span className="text-slate-500">home</span>
          <select className="rounded border border-slate-700 bg-slate-950 px-1 py-0.5" value={home} disabled={locked} onChange={(e) => set(setHome)(e.target.value)} data-testid="edit-home">
            {!homeNow && <option value="">(slot {String(game.home_slot)}, no team)</option>}
            {view.slot_map.map((e) => (
              <option key={String(e.team_id)} value={String(e.team_id)}>
                {e.name} ({e.abbrev})
              </option>
            ))}
          </select>
        </label>
        <label className="flex flex-col">
          <span className="text-slate-500">away</span>
          <select className="rounded border border-slate-700 bg-slate-950 px-1 py-0.5" value={away} disabled={locked} onChange={(e) => set(setAway)(e.target.value)} data-testid="edit-away">
            {!awayNow && <option value="">(slot {String(game.away_slot)}, no team)</option>}
            {view.slot_map.map((e) => (
              <option key={String(e.team_id)} value={String(e.team_id)}>
                {e.name} ({e.abbrev})
              </option>
            ))}
          </select>
        </label>
        <label className="flex flex-col">
          <span className="text-slate-500">month</span>
          <select className="rounded border border-slate-700 bg-slate-950 px-1 py-0.5" value={month} disabled={locked} onChange={(e) => set(setMonth)(e.target.value)} data-testid="edit-month">
            {MONTHS.map((m, i) => (
              <option key={m} value={String(i + 1)}>
                {m}
              </option>
            ))}
          </select>
        </label>
        <label className="flex flex-col">
          <span className="text-slate-500">day</span>
          <input className="w-14 rounded border border-slate-700 bg-slate-950 px-1 py-0.5" value={day} disabled={locked} onChange={(e) => set(setDay)(e.target.value)} inputMode="numeric" data-testid="edit-day" />
        </label>
        <Button
          variant="secondary"
          className="px-2 py-0.5 text-xs"
          disabled={locked}
          onClick={() => {
            const args = { projectId, studioTable, edit: edit() };
            void rv.preview(() => call("schedule_edit_preview", args));
          }}
          data-testid="edit-preview"
        >
          Preview
        </Button>
        <Button
          className="px-2 py-0.5 text-xs"
          disabled={!canApply}
          onClick={() => void rv.apply(projectId, (r) => onApplied(`Game ${String(r.plan.after[0]?.index ?? game.index)} updated: ${r.outcome.edits} cell${r.outcome.edits === 1 ? "" : "s"} written, one undo step.`))}
          data-testid="edit-apply"
        >
          Apply
        </Button>
        <Button variant="ghost" className="px-2 py-0.5 text-xs" disabled={locked} onClick={onCancel}>
          Cancel
        </Button>
      </div>
      {rv.error && <p className="mt-1 text-rose-400">{rv.error}</p>}
      {rv.plan && <PlanSummary plan={rv.plan} label="This edit" />}
    </div>
  );
}

/** Import a CSV/JSON schedule: pick a file, review the key → team mapping, preview, apply. */
function ImportPanel({
  ctxKey,
  ctxRef,
  projectId,
  studioTable,
  view,
  onApplied,
}: Ctx & {
  projectId: number;
  studioTable: string;
  view: ScheduleViewDto;
  onApplied: (message: string) => void;
}) {
  const [path, setPath] = useState<string | null>(null);
  const [format, setFormat] = useState<"csv" | "json">("csv");
  const [overrides, setOverrides] = useState<Record<string, string>>({});
  const rv = useReview<ScheduleImportPlanDto>({ ctxKey, ctxRef }, (p) => p.change);
  const mapping = (o: Record<string, string>): ScheduleTeamMappingArgs[] =>
    Object.entries(o)
      .filter(([, t]) => t !== "")
      .map(([key, t]) => ({ key, team_id: Number(t) }));
  function previewWith(p: string, fmt: "csv" | "json", o: Record<string, string>) {
    const args = { projectId, studioTable, format: fmt, path: p, mapping: mapping(o) };
    void rv.preview(() => call("schedule_import_preview", args));
  }
  async function pick() {
    const started = ctxRef.current;
    const picked = await openDialog({ multiple: false, directory: false, title: "Import schedule", filters: [{ name: "Schedule", extensions: ["csv", "json"] }] });
    // The dialog came back for another context (or was cancelled): nothing is previewed.
    if (typeof picked !== "string" || ctxRef.current !== started || rv.applying) return;
    const fmt = picked.toLowerCase().endsWith(".json") ? "json" : "csv";
    rv.invalidate();
    setFormat(fmt);
    setPath(picked);
    setOverrides({});
    previewWith(picked, fmt, {});
  }
  function remap(key: string, teamId: string) {
    const next = { ...overrides, [key]: teamId };
    rv.invalidate(); // the shown plan no longer describes this mapping
    setOverrides(next);
    if (path) previewWith(path, format, next);
  }
  const plan = rv.plan;
  const unresolved = plan?.keys.filter((k) => k.how === "unmapped" || k.how === "ambiguous") ?? [];
  const locked = rv.applying;
  const canApply = !!plan && plan.change.validation.errors.length === 0 && plan.change.cells > 0 && !locked;
  return (
    <div className="mb-3 rounded-lg border border-slate-800 bg-slate-900/40 p-3 text-xs" data-testid="import-panel">
      <div className="flex flex-wrap items-center gap-2">
        <span className="font-medium text-slate-200">Import a schedule file</span>
        <Button variant="secondary" className="px-2 py-0.5 text-xs" disabled={locked} onClick={() => void pick()}>
          Choose file…
        </Button>
        {path && <span className="truncate font-mono text-slate-400">{path}</span>}
        {path && (
          <Button
            className="px-2 py-0.5 text-xs"
            disabled={!canApply}
            onClick={() => void rv.apply(projectId, (r) => onApplied(`Imported ${r.import?.rows_in_file ?? "?"} games into ${r.plan.table_name}: ${r.outcome.edits} cell${r.outcome.edits === 1 ? "" : "s"} written, one undo step.`))}
            data-testid="import-apply"
          >
            Apply import
          </Button>
        )}
      </div>
      <p className="mt-1 text-slate-500">
        Export a CSV template, edit its games, then preview the import. Dates use month and day.
        {flexibleSchedule(view.table.name) ? " Team IDs identify the clubs; names are included for readability. You can add or remove game rows." : ` The file must contain ${view.games.length} games.`}
      </p>
      {rv.error && <p className="mt-1 text-rose-400">{rv.error}</p>}
      {rv.previewing && <Spinner label="Reading the file…" />}
      {plan && (
        <>
          <div className="mt-2 text-slate-300">
            {plan.rows_in_file} games in the file · {plan.rows_in_table} active rows in {plan.change.table_name} · {plan.keys.length} team keys
            {unresolved.length > 0 && <span className="text-amber-200"> · {unresolved.length} need an explicit destination</span>}
          </div>
          <details className="mt-1" open={unresolved.length > 0}>
            <summary className="cursor-pointer text-slate-400">Team mapping (file key → team on this title)</summary>
            <table className="mt-1 w-full">
              <tbody>
                {plan.keys.map((k) => (
                  <tr key={k.key} className="border-t border-slate-800/60">
                    <td className="py-0.5 pr-2 font-mono">{k.key}</td>
                    <td className="py-0.5 pr-2 text-slate-500">{k.games} games</td>
                    <td className="py-0.5 pr-2">
                      <select
                        className={"rounded border bg-slate-950 px-1 py-0.5 " + (k.how === "unmapped" || k.how === "ambiguous" ? "border-amber-500" : "border-slate-700")}
                        value={overrides[k.key] ?? (k.team_id != null ? String(k.team_id) : "")}
                        disabled={locked}
                        onChange={(e) => remap(k.key, e.target.value)}
                        data-testid={`map-${k.key}`}
                      >
                        <option value="">{k.how === "ambiguous" ? "(ambiguous — choose)" : "(unmapped — choose)"}</option>
                        {view.slot_map.map((e) => (
                          <option key={String(e.team_id)} value={String(e.team_id)}>
                            {e.name} ({e.abbrev})
                          </option>
                        ))}
                      </select>
                    </td>
                    <td className="py-0.5 text-slate-500">{k.how}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </details>
          <PlanSummary plan={plan.change} label="This import" />
        </>
      )}
    </div>
  );
}

/** Generate a balanced round-robin for an explicit topology, or show the exact blocker; or (receipt
 * 34) build the games of each pair from a user-supplied quota matrix — symmetric by construction
 * here (one cell per pair), its total checked against the rows to fill before any preview. */
function GeneratePanel({
  ctxKey,
  ctxRef,
  projectId,
  studioTable,
  view,
  onApplied,
}: Ctx & {
  projectId: number;
  studioTable: string;
  view: ScheduleViewDto;
  onApplied: (message: string) => void;
}) {
  const options = useQuery({
    queryKey: ["schedule-generation-options", projectId, studioTable, view.fingerprint],
    queryFn: () => call("schedule_generation_options", { projectId, studioTable }),
  });
  const o: ScheduleGenerationOptionsDto | undefined = options.data;
  const [participants, setParticipants] = useState<Set<string> | null>(null);
  const [rounds, setRounds] = useState<string>("");
  const [startMonth, setStartMonth] = useState<string>("");
  const [startDay, setStartDay] = useState<string>("");
  const [spacing, setSpacing] = useState<string>("");
  const [mode, setMode] = useState<"round-robin" | "quotas" | "groups">("round-robin");
  /** Receipt 48 — groups mode: group number per participant ("" = none), games inside / across groups. */
  const [groupOf, setGroupOf] = useState<Record<string, string>>({});
  const [groupCount, setGroupCount] = useState<string>("2");
  const [within, setWithin] = useState<string>("");
  const [across, setAcross] = useState<string>("");
  const [groupsOrigin, setGroupsOrigin] = useState<string | null>(null);
  const [stockNote, setStockNote] = useState<string | null>(null);
  /** `per_pair` hosts keyed "lowId|highId" → host team id. */
  const [hosts, setHosts] = useState<Record<string, string>>({});
  /** Games per pair keyed "lowId|highId" (team ids); empty = 0. */
  const [quotas, setQuotas] = useState<Record<string, string>>({});
  /** Host of the extra game of an odd pair count (R123 Q11); "" = not chosen yet. */
  const [oddHome, setOddHome] = useState<"" | "lower_slot" | "higher_slot" | "balance" | "per_pair">("");
  const rv = useReview<ScheduleGenerationPlanDto>({ ctxKey, ctxRef }, (p) => p.change);
  useEffect(() => {
    if (!o) return;
    rv.invalidate();
    setParticipants(new Set(o.members.filter((m) => m.default).map((m) => String(m.team_id))));
    setRounds(o.suggested_rounds != null ? String(o.suggested_rounds) : flexibleSchedule(view.table.name) ? "2" : "");
    setStartMonth(String(Number(o.suggested_start_month) + 1));
    setStartDay(String(Number(o.suggested_start_day) + 1));
    setSpacing(String(o.suggested_days_between_matchdays));
    setQuotas({});
    setGroupOf({});
    setGroupsOrigin(null);
    setStockNote(null);
    setHosts({});
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [o]);
  const chosen = (o?.members ?? []).filter((m) => participants?.has(String(m.team_id)));
  const pairKey = (a: bigint, b: bigint) => (a < b ? `${a}|${b}` : `${b}|${a}`);
  const num = (v: string) => (v.trim() === "" ? 0 : Number(v));
  const quota = (a: bigint, b: bigint) => {
    if (mode === "groups") {
      const ga = groupOf[String(a)] ?? "";
      const gb = groupOf[String(b)] ?? "";
      return ga !== "" && ga === gb ? num(within) : num(across);
    }
    return num(quotas[pairKey(a, b)] ?? "");
  };
  const ungrouped = mode === "groups" ? chosen.filter((m) => (groupOf[String(m.team_id)] ?? "") === "") : [];
  const oddPairs = chosen.flatMap((a, i) => chosen.slice(i + 1).filter((b) => quota(a.team_id, b.team_id) % 2 === 1).map((b) => [a, b] as const));
  const groupsFor = (): number[][] => {
    const byGroup = new Map<string, number[]>();
    chosen.forEach((m) => {
      const g = groupOf[String(m.team_id)] ?? "";
      if (g !== "") byGroup.set(g, [...(byGroup.get(g) ?? []), Number(m.team_id)]);
    });
    return Array.from(byGroup.keys()).sort((x, y) => Number(x) - Number(y)).map((k) => byGroup.get(k)!);
  };
  const fillShippedGroups = async () => {
    rv.invalidate();
    const s = await call("schedule_stock_groups", { projectId, studioTable });
    if (!s.available || !s.groups) {
      setStockNote(s.reason ?? "no shipped groups are recorded for this table");
      return;
    }
    const next: Record<string, string> = {};
    s.groups.groups.forEach((g, k) => g.forEach((t) => { next[String(t)] = String(k + 1); }));
    const ids = new Set(Object.keys(next));
    setParticipants(ids);
    setGroupOf(next);
    setGroupCount(String(s.groups.groups.length));
    setWithin(String(s.groups.within));
    setAcross(String(s.groups.across));
    setGroupsOrigin(s.groups.origin ?? null);
    setStockNote(`Filled from the shipped table (${s.source}, ${s.games} games); you can still change any group or count.`);
  };
  const matrix = (): number[][] => chosen.map((a) => chosen.map((b) => (a.team_id === b.team_id ? 0 : quota(a.team_id, b.team_id))));
  const quotaTotal = chosen.reduce((sum, a, i) => sum + chosen.slice(i + 1).reduce((s, b) => s + quota(a.team_id, b.team_id), 0), 0);
  const anyOdd = chosen.some((a, i) => chosen.slice(i + 1).some((b) => quota(a.team_id, b.team_id) % 2 === 1));
  const quotaBad = chosen.some((a, i) => chosen.slice(i + 1).some((b) => !Number.isInteger(quota(a.team_id, b.team_id)) || quota(a.team_id, b.team_id) < 0));
  const config = (): ScheduleGenerationConfigArgs => {
    const base = {
      participants: mode === "quotas" ? chosen.map((m) => Number(m.team_id)) : Array.from(participants ?? []).map(Number),
      rounds: mode === "quotas" ? 0 : Number(rounds),
      start_month: Number(startMonth) - 1,
      start_day: Number(startDay) - 1,
      days_between_matchdays: Number(spacing),
    };
    const odd = oddHome !== "" ? { odd_home: oddHome } : {};
    const perPair = oddHome === "per_pair" ? { odd_hosts: oddPairs.filter(([a, b]) => hosts[pairKey(a.team_id, b.team_id)]).map(([a, b]) => { const h = hosts[pairKey(a.team_id, b.team_id)]; return h === String(a.team_id) ? { home: Number(a.team_id), away: Number(b.team_id) } : { home: Number(b.team_id), away: Number(a.team_id) }; }) } : {};
    if (mode === "groups") {
      return { ...base, participants: chosen.map((m) => Number(m.team_id)), rounds: 0, groups: { groups: groupsFor(), within: num(within), across: num(across), ...(groupsOrigin ? { origin: groupsOrigin } : {}) }, ...odd, ...perPair };
    }
    return mode === "quotas" ? { ...base, pair_quotas: matrix(), ...odd, ...perPair } : base;
  };
  const set = (f: (v: string) => void) => (v: string) => {
    rv.invalidate();
    f(v);
  };
  const n = participants?.size ?? 0;
  const pairs = useMemo(() => (n * Math.max(0, n - 1)) / 2, [n]);
  const locked = rv.applying;
  const plan = rv.plan;
  const canApply = !!plan && plan.change.validation.errors.length === 0 && plan.change.cells > 0 && !locked;

  if (options.isLoading) return <Spinner label="Reading the table's topology…" />;
  if (options.isError) return <p className="mb-2 text-xs text-rose-400">{String(options.error)}</p>;
  if (!o) return null;
  return (
    <div className="mb-3 rounded-lg border border-slate-800 bg-slate-900/40 p-3 text-xs" data-testid="generate-panel">
      <div className="flex flex-wrap items-center gap-3">
        <div className="font-medium text-slate-200">{mode === "quotas" ? "Generate from pair quotas" : mode === "groups" ? "Generate from groups" : "Generate a balanced round-robin"}</div>
        <label className="flex items-center gap-1 text-slate-400">
          <input type="radio" name={`gen-mode-${studioTable}`} checked={mode === "round-robin"} disabled={locked} onChange={() => { rv.invalidate(); setMode("round-robin"); }} data-testid="gen-mode-round-robin" />
          balanced round-robin
        </label>
        <label className="flex items-center gap-1 text-slate-400">
          <input type="radio" name={`gen-mode-${studioTable}`} checked={mode === "quotas"} disabled={locked} onChange={() => { rv.invalidate(); setMode("quotas"); }} data-testid="gen-mode-quotas" />
          pair quotas (you choose each pair's games)
        </label>
        <label className="flex items-center gap-1 text-slate-400">
          <input type="radio" name={`gen-mode-${studioTable}`} checked={mode === "groups"} disabled={locked} onChange={() => { rv.invalidate(); setMode("groups"); }} data-testid="gen-mode-groups" />
          groups (games inside / across groups)
        </label>
      </div>
      <p className="mt-1 text-slate-500">
        {mode === "groups"
          ? "Put every participant in one group, then set the games per pair inside a group and across groups; Studio turns that into each pair's games and checks the built table against it. The groups are yours unless you fill them from the shipped table, which is offered only for the exact file and league it was recorded from. "
          : mode === "quotas"
          ? "Each pair plays the number of games you enter (the table is symmetric by construction); an odd count gives one club an extra home game, named in the preview; no club plays twice on a generated matchday. The quotas are your choice — no stock format, NHL rule or other title's pattern is filled in. "
          : "Every participant plays every other one the same number of times; venues alternate per pair. "}
        Games involving teams outside the participants are kept. {flexibleSchedule(view.table.name) ? "The number of games follows your selections." : `The schedule must retain ${view.games.length} games.`} Preview the result before applying.
      </p>
      {mode === "round-robin" && o.blocker && (
        <p className="mt-1 rounded border border-amber-800/60 bg-amber-950/30 px-2 py-1 text-amber-200" data-testid="generate-blocker">
          {o.blocker}
        </p>
      )}
      <div className="mt-2 flex flex-wrap items-end gap-2">
        {mode === "round-robin" && (
          <label className="flex flex-col">
            <span className="text-slate-500">round-robins</span>
            <input className="w-16 rounded border border-slate-700 bg-slate-950 px-1 py-0.5" value={rounds} disabled={locked} onChange={(e) => set(setRounds)(e.target.value)} inputMode="numeric" data-testid="gen-rounds" />
          </label>
        )}
        <label className="flex flex-col">
          <span className="text-slate-500">start month</span>
          <select className="rounded border border-slate-700 bg-slate-950 px-1 py-0.5" value={startMonth} disabled={locked} onChange={(e) => set(setStartMonth)(e.target.value)}>
            {MONTHS.map((m, i) => (
              <option key={m} value={String(i + 1)}>
                {m}
              </option>
            ))}
          </select>
        </label>
        <label className="flex flex-col">
          <span className="text-slate-500">start day</span>
          <input className="w-14 rounded border border-slate-700 bg-slate-950 px-1 py-0.5" value={startDay} disabled={locked} onChange={(e) => set(setStartDay)(e.target.value)} inputMode="numeric" data-testid="gen-start-day" />
        </label>
        <label className="flex flex-col">
          <span className="text-slate-500">days between matchdays</span>
          <input className="w-16 rounded border border-slate-700 bg-slate-950 px-1 py-0.5" value={spacing} disabled={locked} onChange={(e) => set(setSpacing)(e.target.value)} inputMode="numeric" />
        </label>
        {mode === "round-robin" ? (
          <span className="text-slate-500">
            {n} participants · {pairs} pairings per round-robin · {rounds.trim() !== "" ? Number(rounds) * pairs : "?"} games
          </span>
        ) : (
          <span className={quotaTotal === o.rows_to_fill && !quotaBad && ungrouped.length === 0 ? "text-emerald-300" : "text-amber-300"} data-testid="gen-quota-total">
            {mode === "groups" && ungrouped.length > 0 ? `${ungrouped.length} participant(s) in no group · ` : ""}
            {n} participants · {quotaTotal}{!flexibleSchedule(view.table.name) && ` of ${o.rows_to_fill}`} games{quotaBad ? " · every cell must be a whole number ≥ 0" : flexibleSchedule(view.table.name) || quotaTotal === o.rows_to_fill ? "" : ` · ${quotaTotal < o.rows_to_fill ? o.rows_to_fill - quotaTotal + " short" : quotaTotal - o.rows_to_fill + " too many"}`}
          </span>
        )}
        <Button
          variant="secondary"
          className="px-2 py-0.5 text-xs"
          disabled={locked || (mode === "quotas" && quotaBad)}
          onClick={() => {
            const args = { projectId, studioTable, config: config() };
            void rv.preview(() => call("schedule_generate_preview", args));
          }}
          data-testid="gen-preview"
        >
          Preview
        </Button>
        <Button
          className="px-2 py-0.5 text-xs"
          disabled={!canApply}
          onClick={() =>
            void rv.apply(projectId, (r) =>
              onApplied(
                `Generated ${r.generation?.generated_rows ?? "?"} games for ${r.generation?.participants ?? "?"} teams in ${r.plan.table_name} (${r.generation?.kept_rows ?? 0} row${r.generation?.kept_rows === 1 ? "" : "s"} kept): one undo step.`,
              ),
            )
          }
          data-testid="gen-apply"
        >
          Apply generation
        </Button>
      </div>
      {mode !== "round-robin" && anyOdd && (
        <label className="mt-2 flex items-center gap-2 text-slate-400">
          <span>some pairs play an odd number of games — the extra home game goes to</span>
          <select
            className="rounded border border-slate-700 bg-slate-950 px-1 py-0.5"
            value={oddHome}
            disabled={locked}
            onChange={(e) => {
              rv.invalidate();
              setOddHome(e.target.value as typeof oddHome);
            }}
            data-testid="gen-odd-home"
          >
            <option value="">choose…</option>
            <option value="lower_slot">the club at the lower league slot</option>
            <option value="higher_slot">the club at the higher league slot</option>
            <option value="balance">whichever has fewer home games so far (pairs in order; ties: lower slot)</option>
            <option value="per_pair">I choose for each pair</option>
          </select>
        </label>
      )}
      {mode !== "round-robin" && anyOdd && oddHome === "per_pair" && (
        <details className="mt-1" open data-testid="gen-odd-hosts">
          <summary className="cursor-pointer text-slate-400">
            Host of each odd pair ({oddPairs.filter(([a, b]) => hosts[pairKey(a.team_id, b.team_id)]).length} of {oddPairs.length} chosen)
          </summary>
          <div className="mt-1 grid max-h-60 gap-x-4 overflow-auto sm:grid-cols-2">
            {oddPairs.map(([a, b]) => {
              const k = pairKey(a.team_id, b.team_id);
              return (
                <label key={k} className="flex items-center gap-1">
                  <span className="w-40 truncate">{a.abbrev} v {b.abbrev}</span>
                  <select className="rounded border border-slate-700 bg-slate-950 px-1 py-0.5" value={hosts[k] ?? ""} disabled={locked} aria-label={`Host of the extra ${a.name} v ${b.name} game`} onChange={(e) => { rv.invalidate(); setHosts((h) => ({ ...h, [k]: e.target.value })); }}>
                    <option value="">choose…</option>
                    <option value={String(a.team_id)}>{a.name}</option>
                    <option value={String(b.team_id)}>{b.name}</option>
                  </select>
                </label>
              );
            })}
          </div>
        </details>
      )}
      {mode === "groups" && (
        <div className="mt-2 rounded border border-slate-800 p-2" data-testid="gen-groups">
          <div className="flex flex-wrap items-end gap-2">
            <label className="flex flex-col">
              <span className="text-slate-500">groups</span>
              <input className="w-12 rounded border border-slate-700 bg-slate-950 px-1 py-0.5" value={groupCount} disabled={locked} onChange={(e) => set(setGroupCount)(e.target.value)} inputMode="numeric" data-testid="gen-group-count" />
            </label>
            <label className="flex flex-col">
              <span className="text-slate-500">games per pair inside a group</span>
              <input className="w-16 rounded border border-slate-700 bg-slate-950 px-1 py-0.5" value={within} disabled={locked} onChange={(e) => { setGroupsOrigin(null); set(setWithin)(e.target.value); }} inputMode="numeric" data-testid="gen-within" />
            </label>
            <label className="flex flex-col">
              <span className="text-slate-500">games per pair across groups</span>
              <input className="w-16 rounded border border-slate-700 bg-slate-950 px-1 py-0.5" value={across} disabled={locked} onChange={(e) => { setGroupsOrigin(null); set(setAcross)(e.target.value); }} inputMode="numeric" data-testid="gen-across" />
            </label>
            <Button variant="secondary" className="px-2 py-0.5 text-xs" disabled={locked} onClick={() => void fillShippedGroups()} data-testid="gen-stock-groups">
              Use the shipped groups
            </Button>
          </div>
          {stockNote && <p className="mt-1 text-slate-400" data-testid="gen-stock-note">{stockNote}</p>}
          <div className="mt-2 grid gap-x-4 sm:grid-cols-3">
            {chosen.map((m) => (
              <label key={String(m.team_id)} className="flex items-center gap-1">
                <span className="w-40 truncate" title={m.name}>{m.name}</span>
                <select
                  className="rounded border border-slate-700 bg-slate-950 px-1 py-0.5"
                  value={groupOf[String(m.team_id)] ?? ""}
                  disabled={locked}
                  aria-label={`Group of ${m.name}`}
                  onChange={(e) => { rv.invalidate(); setGroupsOrigin(null); setGroupOf((g) => ({ ...g, [String(m.team_id)]: e.target.value })); }}
                  data-testid={`gen-group-of-${m.team_id}`}
                >
                  <option value="">no group</option>
                  {Array.from({ length: Math.max(1, Math.min(16, Number(groupCount) || 1)) }, (_, k) => (
                    <option key={k} value={String(k + 1)}>group {k + 1}</option>
                  ))}
                </select>
              </label>
            ))}
          </div>
        </div>
      )}
      {mode === "quotas" && <QuotaGrid chosen={chosen} raw={(a, b) => quotas[pairKey(a, b)] ?? ""} locked={locked} onFill={(v) => { rv.invalidate(); const next: Record<string, string> = {}; chosen.forEach((a, i) => chosen.slice(i + 1).forEach((b) => { next[pairKey(a.team_id, b.team_id)] = v; })); setQuotas(next); }} onSet={(a, b, v) => { rv.invalidate(); setQuotas((q) => ({ ...q, [pairKey(a, b)]: v })); }} />}
      <details className="mt-2">
        <summary className="cursor-pointer text-slate-400">Participants ({n} of {o.members.length})</summary>
        <div className="mt-1 grid gap-x-4 sm:grid-cols-3">
          {o.members.map((m) => (
            <label key={String(m.team_id)} className="flex items-center gap-1">
              <input
                type="checkbox"
                disabled={locked}
                checked={participants?.has(String(m.team_id)) ?? false}
                onChange={(e) => {
                  const next = new Set(participants ?? []);
                  if (e.target.checked) next.add(String(m.team_id));
                  else next.delete(String(m.team_id));
                  rv.invalidate();
                  setParticipants(next);
                }}
              />
              <span>
                {m.name} <span className="text-slate-500">({m.games} now)</span>
              </span>
            </label>
          ))}
        </div>
      </details>
      {o.notes.length > 0 && (
        <ul className="mt-1 list-disc pl-5 text-slate-500">
          {o.notes.map((n, i) => (
            <li key={i}>{n}</li>
          ))}
        </ul>
      )}
      {rv.error && <p className="mt-1 text-rose-400">{rv.error}</p>}
      {plan && <PlanSummary plan={plan.change} label={plan.rounds === 0 ? `${plan.participants} teams, ${plan.generated_rows} games from pair quotas, ${plan.matchdays} matchdays` : `${plan.participants} teams × ${plan.rounds} round-robin(s), ${plan.matchdays} matchdays`} />}
    </div>
  );
}

/** One editable cell per pair (upper triangle); the mirrored cell shows the same number. */
function QuotaGrid({
  chosen,
  raw,
  locked,
  onFill,
  onSet,
}: {
  chosen: Array<{ team_id: bigint; abbrev: string; name: string }>;
  raw: (a: bigint, b: bigint) => string;
  locked: boolean;
  onFill: (v: string) => void;
  onSet: (a: bigint, b: bigint, v: string) => void;
}) {
  const [fill, setFill] = useState("4");
  return (
    <div className="mt-2" data-testid="gen-quota-grid">
      <div className="mb-1 flex items-center gap-2 text-slate-400">
        <span>set every pair to</span>
        <input className="w-12 rounded border border-slate-700 bg-slate-950 px-1 py-0.5" value={fill} disabled={locked} onChange={(e) => setFill(e.target.value)} inputMode="numeric" data-testid="gen-quota-fill-value" />
        <Button variant="secondary" className="px-2 py-0.5 text-xs" disabled={locked} onClick={() => onFill(fill)} data-testid="gen-quota-fill">
          Fill
        </Button>
        <span className="text-slate-500">then adjust single pairs (row club v column club)</span>
      </div>
      <div className="max-h-72 overflow-auto">
        <table className="border-collapse font-mono text-[11px]">
          <thead>
            <tr>
              <th />
              {chosen.map((b) => (
                <th key={String(b.team_id)} className="px-1 text-slate-400" title={b.name}>
                  {b.abbrev}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {chosen.map((a, i) => (
              <tr key={String(a.team_id)}>
                <th className="pr-1 text-right text-slate-400" title={a.name}>
                  {a.abbrev}
                </th>
                {chosen.map((b, j) => (
                  <td key={String(b.team_id)} className="p-0">
                    {i === j ? (
                      <span className="block w-8 text-center text-slate-600">—</span>
                    ) : j > i ? (
                      <input
                        className="w-8 rounded border border-slate-800 bg-slate-950 px-0.5 text-center"
                        value={raw(a.team_id, b.team_id)}
                        disabled={locked}
                        onChange={(e) => onSet(a.team_id, b.team_id, e.target.value)}
                        inputMode="numeric"
                        aria-label={`${a.name} v ${b.name}`}
                        data-testid={`gen-quota-${a.team_id}-${b.team_id}`}
                      />
                    ) : (
                      <span className="block w-8 text-center text-slate-500">{raw(a.team_id, b.team_id) || "0"}</span>
                    )}
                  </td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

/// A slot without a bound team: its tournament browse label (R115) when one exists — the raw slot id
/// stays behind the tooltip — else the bare slot number, as before.
function SlotLabel({ label, slot }: { label: string | null; slot: bigint | number }) {
  if (label) {
    return (
      <span className="text-sky-200/80" title={`slot ${String(slot)}`} data-testid="schedule-slot-label">
        {label}
      </span>
    );
  }
  return <span className="text-amber-300/80">slot {String(slot)}</span>;
}
