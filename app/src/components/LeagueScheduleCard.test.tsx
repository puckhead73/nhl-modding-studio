// @vitest-environment jsdom
//
// P1-A / R1 F2 — the schedule card's request ownership, driven with deferred client responses.
// Component evidence only (jsdom): the server-side validation and review binding are the Rust
// suites' job; here we check what is displayed and exactly what is dispatched when responses
// arrive late, out of order, or for a context that is gone.
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ScheduleViewDto } from "../bindings/ScheduleViewDto";
import type { ScheduleChangePlanDto } from "../bindings/ScheduleChangePlanDto";
import type { ScheduleTableDto } from "../bindings/ScheduleTableDto";

type Deferred<T> = { promise: Promise<T>; resolve: (v: T) => void; reject: (e: unknown) => void };
function deferred<T>(): Deferred<T> {
  let resolve!: (v: T) => void;
  let reject!: (e: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

const calls: Array<{ cmd: string; args: any }> = [];
const handlers: Record<string, (args: any) => unknown> = {};
vi.mock("../lib/client", () => ({
  call: (cmd: string, args: unknown) => {
    calls.push({ cmd, args });
    const h = handlers[cmd];
    if (!h) return Promise.reject(new Error(`unmocked command ${cmd}`));
    return Promise.resolve().then(() => h(args));
  },
}));
const updates: Array<{ project_id: number }> = [];
vi.mock("../lib/project", () => ({
  useProject: () => ({ update: (s: { project_id: number }) => updates.push(s) }),
}));
let dialogOpen: Deferred<string | null> | null = null;
vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: () => (dialogOpen ? dialogOpen.promise : Promise.resolve("C:\\tmp\\nhlschedule.csv")),
  save: () => Promise.resolve("C:\\tmp\\out.csv"),
}));

import { LeagueScheduleCard } from "./LeagueScheduleCard";

const tableA: ScheduleTableDto = { studio_table: "T", name: "swedishschedule", rows: 2, max_records: 2, league_code: 2n, league_label: "SHL", league_match: "stem", role: "source", shown_with_league: null };
const tableB: ScheduleTableDto = { studio_table: "U", name: "germanschedule", rows: 2, max_records: 2, league_code: 4n, league_label: "DEL", league_match: "stem", role: "source", shown_with_league: null };
const tableA2: ScheduleTableDto = { ...tableA, studio_table: "T2", name: "swedishscheduledebug", role: "debug" };
// R115: a tournament template shown with league 2 for display only — no league code, no slot map.
const tableW: ScheduleTableDto = { studio_table: "W", name: "wcschedule", rows: 2, max_records: 20, league_code: null, league_label: null, league_match: "stem \"wc\": no league mapping known", role: "source", shown_with_league: 2n };

function view(over: Partial<ScheduleViewDto> = {}): ScheduleViewDto {
  return {
    table: tableA,
    fields: [["home", "h"], ["away", "a"], ["day", "d"], ["month", "m"], ["status", "s"]],
    field_widths: [["home", 4]],
    slot_map: [
      { slot: 0n, team_id: 100n, name: "Frölunda", abbrev: "FHC", key: "FHC", global_index: 62n },
      { slot: 1n, team_id: 101n, name: "Färjestad", abbrev: "FBK", key: "FBK", global_index: 63n },
    ],
    fingerprint: "abc",
    games: [
      { record: 0, index: 0n, month: 8n, day: 15n, date: "Sep 16", round: 0n, status: 0n, home_slot: 0n, away_slot: 1n, home: "Frölunda", away: "Färjestad", round_label: null, home_label: null, away_label: null },
      { record: 1, index: 1n, month: 8n, day: 17n, date: "Sep 18", round: 0n, status: 0n, home_slot: 1n, away_slot: 0n, home: "Färjestad", away: "Frölunda", round_label: null, home_label: null, away_label: null },
    ],
    slots: [
      { slot: 0n, team: "Frölunda", team_id: 100n, games: 2 },
      { slot: 1n, team: "Färjestad", team_id: 101n, games: 2 },
    ],
    unresolved_slots: [],
    findings: [],
    tournament_note: null,
    first_date: "Sep 16",
    last_date: "Sep 18",
    ...over,
  };
}
function plan(over: Partial<ScheduleChangePlanDto>): ScheduleChangePlanDto {
  return {
    ticket: "ws-a/1",
    studio_table: "T",
    table_name: "swedishschedule",
    league_label: "SHL",
    rows_changed: 1,
    cells: 2,
    before: [],
    after: [],
    validation: { errors: [], warnings: [], games_per_slot: [], first_date: "Sep 16", last_date: "Sep 18" },
    fingerprint: "abc",
    notes: [],
    ...over,
  };
}
const applied = (ticket: string) => ({ ticket, kind: "edit", plan: plan({ ticket }), import: null, generation: null, outcome: { edits: 2, repairs: [] }, view: view() });

function mount(props: { projectId: number; leagueCode: number }) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } });
  const ui = (p: { projectId: number; leagueCode: number }) => (
    <QueryClientProvider client={qc}>
      <LeagueScheduleCard projectId={p.projectId} leagueCode={p.leagueCode} />
    </QueryClientProvider>
  );
  const r = render(ui(props));
  return { rerender: (p: { projectId: number; leagueCode: number }) => r.rerender(ui(p)), unmount: r.unmount };
}
const applyBtn = () => screen.getByTestId("edit-apply") as HTMLButtonElement;
const of = (cmd: string) => calls.filter((c) => c.cmd === cmd);

describe("LeagueScheduleCard (P1-A R1 ownership)", () => {
  beforeEach(() => {
    calls.length = 0;
    updates.length = 0;
    dialogOpen = null;
    for (const k of Object.keys(handlers)) delete handlers[k];
    handlers.schedule_tables = (a) => (a.projectId === 7 ? [tableA, tableA2, tableB] : [{ ...tableA, studio_table: "X" }]);
    handlers.schedule_view = (a) => view({ table: { ...tableA, studio_table: a.studioTable } });
    handlers.project_summary = (a) => ({ project_id: a.projectId, dirty: true });
  });
  afterEach(() => cleanup());

  it("shows a tournament template on its league's page, labelled and read-only (R115)", async () => {
    handlers.schedule_tables = () => [tableA, tableW];
    handlers.schedule_view = (a) =>
      a.studioTable === "W"
        ? view({
            table: tableW,
            slot_map: [],
            games: [
              { record: 0, index: 0n, month: 8n, day: 17n, date: "Sep 18", round: 0n, status: null, home_slot: 0n, away_slot: 7n, home: null, away: null, round_label: "group stage", home_label: "CAN", away_label: "USA" },
              { record: 1, index: 1n, month: 8n, day: 27n, date: "Sep 28", round: 3n, status: null, home_slot: 8n, away_slot: 8n, home: null, away: null, round_label: "knockout round — teams to be decided", home_label: "to be decided", away_label: "to be decided" },
            ],
            slots: [],
            tournament_note: "Tournament template (research R115): … Browse-only.",
          })
        : view({ table: { ...tableA, studio_table: a.studioTable } });
    mount({ projectId: 7, leagueCode: 2 });
    await screen.findByTestId("schedule-summary");
    fireEvent.click(screen.getByText(/^wcschedule /));
    await waitFor(() => expect(screen.getByTestId("schedule-tournament-note")).toBeTruthy());
    const labels = screen.getAllByTestId("schedule-slot-label").map((e) => e.textContent);
    expect(labels).toEqual(["CAN", "USA", "to be decided", "to be decided"]);
    expect(screen.getByText("group stage")).toBeTruthy();
    expect(screen.queryByTestId("edit-game-0")).toBeNull();
    expect(screen.getByText("Generate…").closest("button")?.disabled).toBe(true);
  });

  async function openEditor() {
    await screen.findByTestId("schedule-summary");
    fireEvent.click(screen.getByTestId("edit-game-0"));
    await screen.findByTestId("game-editor");
  }

  it("applies the held review id (not the inputs); a preview alone writes nothing", async () => {
    handlers.schedule_edit_preview = () => plan({ ticket: "ws-a/41" });
    handlers.schedule_edit_apply = () => {
      throw new Error("old command must not be used");
    };
    handlers.schedule_apply_review = (a) => applied(a.ticket);
    mount({ projectId: 7, leagueCode: 2 });
    await openEditor();
    fireEvent.change(screen.getByTestId("edit-home"), { target: { value: "101" } });
    fireEvent.change(screen.getByTestId("edit-away"), { target: { value: "100" } });
    expect(applyBtn().disabled).toBe(true);
    fireEvent.click(screen.getByTestId("edit-preview"));
    await screen.findByTestId("schedule-plan");
    expect(of("schedule_edit_preview")[0].args).toEqual({ projectId: 7, studioTable: "T", edit: { record: 0, home_team_id: 101, away_team_id: 100, month: 8, day: 15 } });
    expect(of("schedule_apply_review").length).toBe(0);
    fireEvent.click(applyBtn());
    await screen.findByTestId("schedule-notice");
    expect(of("schedule_apply_review").map((c) => c.args)).toEqual([{ projectId: 7, ticket: "ws-a/41" }]);
    expect(updates).toEqual([{ project_id: 7, dirty: true }]);
  });

  it("a preview that resolves after an input change is dropped: no plan, Apply disabled, next preview sends the new inputs", async () => {
    const first = deferred<ScheduleChangePlanDto>();
    let n = 0;
    handlers.schedule_edit_preview = () => (++n === 1 ? first.promise : plan({ ticket: "ws-a/2" }));
    mount({ projectId: 7, leagueCode: 2 });
    await openEditor();
    fireEvent.click(screen.getByTestId("edit-preview"));
    fireEvent.change(screen.getByTestId("edit-day"), { target: { value: "20" } });
    await act(async () => {
      first.resolve(plan({ ticket: "ws-a/1" }));
      await first.promise;
    });
    expect(screen.queryByTestId("schedule-plan")).toBeNull();
    expect(applyBtn().disabled).toBe(true);
    fireEvent.click(screen.getByTestId("edit-preview"));
    await screen.findByTestId("schedule-plan");
    expect(screen.getByTestId("schedule-plan").getAttribute("data-review")).toBe("ws-a/2");
    expect(of("schedule_edit_preview")[1].args.edit.day).toBe(19);
  });

  it("previews resolving out of order: only the latest request's plan is shown", async () => {
    const d1 = deferred<ScheduleChangePlanDto>();
    const d2 = deferred<ScheduleChangePlanDto>();
    let n = 0;
    handlers.schedule_edit_preview = () => (++n === 1 ? d1.promise : d2.promise);
    mount({ projectId: 7, leagueCode: 2 });
    await openEditor();
    fireEvent.click(screen.getByTestId("edit-preview"));
    fireEvent.change(screen.getByTestId("edit-day"), { target: { value: "21" } });
    fireEvent.click(screen.getByTestId("edit-preview"));
    await act(async () => {
      d2.resolve(plan({ ticket: "ws-a/22" }));
      await d2.promise;
    });
    expect(screen.getByTestId("schedule-plan").getAttribute("data-review")).toBe("ws-a/22");
    await act(async () => {
      d1.resolve(plan({ ticket: "ws-a/11" }));
      await d1.promise;
    });
    expect(screen.getByTestId("schedule-plan").getAttribute("data-review")).toBe("ws-a/22");
    fireEvent.click(applyBtn());
    handlers.schedule_apply_review = (a) => applied(a.ticket);
    await waitFor(() => expect(of("schedule_apply_review").length).toBe(1));
    expect(of("schedule_apply_review")[0].args.ticket).toBe("ws-a/22");
  });

  it("double Apply dispatches exactly one apply; inputs lock while it runs", async () => {
    handlers.schedule_edit_preview = () => plan({ ticket: "ws-a/5" });
    const ap = deferred<ReturnType<typeof applied>>();
    handlers.schedule_apply_review = () => ap.promise;
    mount({ projectId: 7, leagueCode: 2 });
    await openEditor();
    fireEvent.click(screen.getByTestId("edit-preview"));
    await screen.findByTestId("schedule-plan");
    fireEvent.click(applyBtn());
    fireEvent.click(applyBtn());
    await waitFor(() => expect(of("schedule_apply_review").length).toBe(1));
    expect((screen.getByTestId("edit-day") as HTMLInputElement).disabled).toBe(true);
    expect(applyBtn().disabled).toBe(true);
    await act(async () => {
      ap.resolve(applied("ws-a/5"));
      await ap.promise;
    });
    await screen.findByTestId("schedule-notice");
    expect(of("schedule_apply_review").length).toBe(1);
  });

  it("project switch during preview / apply / summary refresh: no stale plan, no foreign summary update, no notice", async () => {
    const pv = deferred<ScheduleChangePlanDto>();
    handlers.schedule_edit_preview = () => pv.promise;
    const { rerender } = mount({ projectId: 7, leagueCode: 2 });
    await openEditor();
    fireEvent.click(screen.getByTestId("edit-preview"));
    rerender({ projectId: 8, leagueCode: 2 });
    await screen.findByTestId("schedule-summary");
    await act(async () => {
      pv.resolve(plan({ ticket: "ws-a/9" }));
      await pv.promise;
    });
    expect(screen.queryByTestId("schedule-plan")).toBeNull();
    expect(screen.queryByTestId("game-editor")).toBeNull();
    // Now an apply on project 8 whose summary refresh completes after switching back to 7.
    handlers.schedule_edit_preview = () => plan({ ticket: "ws-a/10", studio_table: "X" });
    const sum = deferred<{ project_id: number; dirty: boolean }>();
    handlers.project_summary = (a) => (a.projectId === 8 ? sum.promise : { project_id: a.projectId, dirty: false });
    handlers.schedule_apply_review = (a) => applied(a.ticket);
    fireEvent.click(screen.getByTestId("edit-game-0"));
    await screen.findByTestId("game-editor");
    fireEvent.click(screen.getByTestId("edit-preview"));
    await screen.findByTestId("schedule-plan");
    fireEvent.click(applyBtn());
    await waitFor(() => expect(of("schedule_apply_review").length).toBe(1));
    expect(of("schedule_apply_review")[0].args).toEqual({ projectId: 8, ticket: "ws-a/10" });
    rerender({ projectId: 7, leagueCode: 2 });
    await screen.findByTestId("schedule-summary");
    await act(async () => {
      sum.resolve({ project_id: 8, dirty: true });
      await sum.promise;
    });
    expect(updates).toEqual([]);
    expect(screen.queryByTestId("schedule-notice")).toBeNull();
  });

  it("league switch resets the picked table and drops a pending preview", async () => {
    const pv = deferred<ScheduleChangePlanDto>();
    handlers.schedule_edit_preview = () => pv.promise;
    const { rerender } = mount({ projectId: 7, leagueCode: 2 });
    await screen.findByTestId("schedule-summary");
    fireEvent.click(screen.getByText(/swedishscheduledebug/));
    await waitFor(() => expect(of("schedule_view").some((c) => c.args.studioTable === "T2")).toBe(true));
    await waitFor(() => expect(screen.queryByTestId("edit-game-0")).not.toBeNull());
    fireEvent.click(screen.getByTestId("edit-game-0"));
    await screen.findByTestId("game-editor");
    fireEvent.click(screen.getByTestId("edit-preview"));
    expect(of("schedule_edit_preview")[0].args.studioTable).toBe("T2");
    rerender({ projectId: 7, leagueCode: 4 });
    await waitFor(() => expect(of("schedule_view").some((c) => c.args.studioTable === "U")).toBe(true));
    await act(async () => {
      pv.resolve(plan({ ticket: "ws-a/3" }));
      await pv.promise;
    });
    expect(screen.queryByTestId("schedule-plan")).toBeNull();
    expect(screen.queryByTestId("game-editor")).toBeNull();
    expect(of("schedule_view").some((c) => c.args.studioTable === "T2" && of("schedule_view").indexOf(c) > of("schedule_view").findIndex((d) => d.args.studioTable === "U"))).toBe(false);
  });

  it("import: a mapping change while a preview is pending drops the old plan and applies the new review", async () => {
    const first = deferred<any>();
    let n = 0;
    const key = (explicit: boolean) => [
      { key: "FHC", team_id: 100n, team: "Frölunda", slot: 0n, how: "auto", games: 2 },
      { key: "club9", team_id: explicit ? 101n : null, team: explicit ? "Färjestad" : null, slot: explicit ? 1n : null, how: explicit ? "explicit" : "unmapped", games: 2 },
    ];
    handlers.schedule_import_preview = (args) => {
      n += 1;
      if (n === 1) return first.promise;
      const explicit = (args.mapping as Array<{ key: string }>).some((m) => m.key === "club9");
      return { change: plan({ ticket: "ws-a/7" + String(n), cells: explicit ? 4 : 0 }), keys: key(explicit), rows_in_file: 2, rows_in_table: 2 };
    };
    handlers.schedule_apply_review = (a) => ({ ...applied(a.ticket), kind: "import", import: { change: plan({}), keys: [], rows_in_file: 2, rows_in_table: 2 } });
    mount({ projectId: 7, leagueCode: 2 });
    await screen.findByTestId("schedule-summary");
    fireEvent.click(screen.getByText("Import…"));
    fireEvent.click(screen.getByText("Choose file…"));
    await waitFor(() => expect(n).toBe(1));
    // First response lands: the unmapped key needs a destination.
    await act(async () => {
      first.resolve({ change: plan({ ticket: "ws-a/71", cells: 0, validation: { errors: ["1 team key(s) in the file match no member of SHL — map each one explicitly: club9"], warnings: [], games_per_slot: [], first_date: null, last_date: null } }), keys: key(false), rows_in_file: 2, rows_in_table: 2 });
      await first.promise;
    });
    await screen.findByText(/map each one explicitly/);
    const importApply = () => screen.getByTestId("import-apply") as HTMLButtonElement;
    expect(importApply().disabled).toBe(true);
    // Mapping change: the plan is cleared at once and the re-preview carries the mapping.
    fireEvent.change(screen.getByTestId("map-club9"), { target: { value: "101" } });
    await waitFor(() => expect(n).toBe(2));
    const last = of("schedule_import_preview").pop()!;
    expect(last.args).toEqual({ projectId: 7, studioTable: "T", format: "csv", path: "C:\\tmp\\nhlschedule.csv", mapping: [{ key: "club9", team_id: 101 }] });
    await waitFor(() => expect(importApply().disabled).toBe(false));
    expect(screen.getByTestId("schedule-plan").getAttribute("data-review")).toBe("ws-a/72");
    fireEvent.click(importApply());
    await screen.findByTestId("schedule-notice");
    expect(of("schedule_apply_review").map((c) => c.args)).toEqual([{ projectId: 7, ticket: "ws-a/72" }]);
  });

  it("file dialog returning after a table switch previews nothing for the old context", async () => {
    dialogOpen = deferred<string | null>();
    handlers.schedule_import_preview = () => ({ change: plan({}), keys: [], rows_in_file: 2, rows_in_table: 2 });
    mount({ projectId: 7, leagueCode: 2 });
    await screen.findByTestId("schedule-summary");
    fireEvent.click(screen.getByText("Import…"));
    fireEvent.click(screen.getByText("Choose file…"));
    fireEvent.click(screen.getByText(/swedishscheduledebug/));
    await waitFor(() => expect(of("schedule_view").some((c) => c.args.studioTable === "T2")).toBe(true));
    await act(async () => {
      dialogOpen!.resolve("C:\\tmp\\late.csv");
      await dialogOpen!.promise;
    });
    expect(of("schedule_import_preview").length).toBe(0);
    expect(screen.queryByTestId("import-panel")).toBeNull();
  });

  it("generate from pair quotas: the grid is symmetric, the total is checked, the matrix is sent and its review applied (receipt 34)", async () => {
    handlers.schedule_generation_options = () => ({
      members: [
        { team_id: 100n, slot: 0n, name: "Frölunda", abbrev: "FHC", games: 3, default: true },
        { team_id: 101n, slot: 1n, name: "Färjestad", abbrev: "FBK", games: 3, default: true },
        { team_id: 102n, slot: 2n, name: "Luleå", abbrev: "LHF", games: 2, default: true },
      ],
      kept_rows: 0,
      rows_to_fill: 4,
      pairs: 3,
      suggested_rounds: null,
      suggested_start_month: 8n,
      suggested_start_day: 15n,
      suggested_days_between_matchdays: 2,
      matchdays_per_round: 3,
      blocker: "4 games for 3 teams is not a whole number of round-robins",
      notes: [],
    });
    handlers.schedule_generate_preview = () => ({ change: plan({ ticket: "ws-a/91", rows_changed: 4, cells: 8 }), participants: 3, rounds: 0, matchdays: 3, generated_rows: 4, kept_rows: 0, games_per_participant: 3 });
    handlers.schedule_apply_review = (a) => ({ ...applied(a.ticket), kind: "generate", generation: { change: plan({}), participants: 3, rounds: 0, matchdays: 3, generated_rows: 4, kept_rows: 0, games_per_participant: 3 } });
    mount({ projectId: 7, leagueCode: 2 });
    await screen.findByTestId("schedule-summary");
    fireEvent.click(screen.getByText("Generate…"));
    await screen.findByTestId("generate-blocker");
    fireEvent.click(screen.getByTestId("gen-mode-quotas"));
    // The round-robin blocker does not apply to quotas; the grid and the running total appear.
    expect(screen.queryByTestId("generate-blocker")).toBeNull();
    expect(screen.queryByTestId("gen-rounds")).toBeNull();
    await screen.findByTestId("gen-quota-grid");
    fireEvent.change(screen.getByTestId("gen-quota-fill-value"), { target: { value: "1" } });
    fireEvent.click(screen.getByTestId("gen-quota-fill"));
    expect(screen.getByTestId("gen-quota-total").textContent).toMatch(/3 participants · 3 of 4 games · 1 short/);
    fireEvent.change(screen.getByTestId("gen-quota-100-101"), { target: { value: "x" } });
    expect(screen.getByTestId("gen-quota-total").textContent).toMatch(/whole number/);
    expect((screen.getByTestId("gen-preview") as HTMLButtonElement).disabled).toBe(true);
    fireEvent.change(screen.getByTestId("gen-quota-100-101"), { target: { value: "2" } });
    expect(screen.getByTestId("gen-quota-total").textContent).toMatch(/^3 participants · 4 of 4 games$/);
    // Odd pairs: the host rule must be chosen (R123 Q11); it is sent with the matrix.
    fireEvent.change(screen.getByTestId("gen-odd-home"), { target: { value: "lower_slot" } });
    fireEvent.click(screen.getByTestId("gen-preview"));
    await screen.findByTestId("schedule-plan");
    expect(of("schedule_generate_preview")[0].args.config).toEqual({
      participants: [100, 101, 102],
      rounds: 0,
      start_month: 8,
      start_day: 15,
      days_between_matchdays: 2,
      pair_quotas: [
        [0, 2, 1],
        [2, 0, 1],
        [1, 1, 0],
      ],
      odd_home: "lower_slot",
    });
    expect(screen.getByText(/3 teams, 4 games from pair quotas, 3 matchdays/)).toBeTruthy();
    // Editing a cell invalidates the preview; the old review is never applied.
    fireEvent.change(screen.getByTestId("gen-quota-101-102"), { target: { value: "2" } });
    fireEvent.change(screen.getByTestId("gen-quota-100-102"), { target: { value: "0" } });
    expect(screen.queryByTestId("schedule-plan")).toBeNull();
    fireEvent.click(screen.getByTestId("gen-preview"));
    await screen.findByTestId("schedule-plan");
    fireEvent.click(screen.getByTestId("gen-apply"));
    await screen.findByTestId("schedule-notice");
    expect(of("schedule_apply_review").map((c) => c.args)).toEqual([{ projectId: 7, ticket: "ws-a/91" }]);
  });

  it("generate: groups — the shipped groups fill only on an exact match, per-pair hosts are sent (receipt 48)", async () => {
    handlers.schedule_generation_options = () => ({
      members: [
        { team_id: 100n, slot: 0n, name: "Frölunda", abbrev: "FHC", games: 3, default: true },
        { team_id: 101n, slot: 1n, name: "Färjestad", abbrev: "FBK", games: 3, default: true },
        { team_id: 102n, slot: 2n, name: "Luleå", abbrev: "LHF", games: 3, default: true },
        { team_id: 103n, slot: 3n, name: "Skellefteå", abbrev: "SKE", games: 3, default: true },
      ],
      kept_rows: 0,
      rows_to_fill: 8,
      pairs: 6,
      suggested_rounds: 1,
      suggested_start_month: 8n,
      suggested_start_day: 15n,
      suggested_days_between_matchdays: 2,
      matchdays_per_round: 3,
      blocker: null,
      notes: [],
    });
    let stock: any = { available: false, groups: null, group_names: [], reason: "the opened file is not one of the recorded sources for swedishschedule" };
    handlers.schedule_stock_groups = () => stock;
    handlers.schedule_generate_preview = () => ({ change: plan({ ticket: "ws-a/92", rows_changed: 6, cells: 12 }), participants: 4, rounds: 0, matchdays: 3, generated_rows: 6, kept_rows: 0, games_per_participant: 3 });
    mount({ projectId: 7, leagueCode: 2 });
    await screen.findByTestId("schedule-summary");
    fireEvent.click(screen.getByText("Generate…"));
    await screen.findByTestId("gen-mode-groups");
    fireEvent.click(screen.getByTestId("gen-mode-groups"));
    // No exact match: the reason is shown, nothing is filled.
    fireEvent.click(screen.getByTestId("gen-stock-groups"));
    expect((await screen.findByTestId("gen-stock-note")).textContent).toMatch(/not one of the recorded sources/);
    expect(screen.getByTestId("gen-quota-total").textContent).toMatch(/4 participant\(s\) in no group/);
    // An exact match fills the groups and counts (2 groups of 2: within 2, across 1 → 2 × 2 + 4 × 1 = 8 games).
    stock = { available: true, groups: { groups: [[100n, 101n], [102n, 103n]], within: 2, across: 1, origin: "the shipped swedishschedule groups of test" }, group_names: [["Frölunda", "Färjestad"], ["Luleå", "Skellefteå"]], source: "test", games: 8n, reason: null };
    fireEvent.click(screen.getByTestId("gen-stock-groups"));
    await waitFor(() => expect(screen.getByTestId("gen-quota-total").textContent).toMatch(/^4 participants · 8 of 8 games$/));
    expect((screen.getByTestId("gen-group-of-102") as HTMLSelectElement).value).toBe("2");
    // Across pairs are odd: choose per pair and name each host.
    fireEvent.change(screen.getByTestId("gen-odd-home"), { target: { value: "per_pair" } });
    const pickers = await screen.findAllByLabelText(/Host of the extra/);
    expect(pickers).toHaveLength(4);
    pickers.forEach((el) => fireEvent.change(el, { target: { value: (el as HTMLSelectElement).options[2].value } }));
    fireEvent.click(screen.getByTestId("gen-preview"));
    await screen.findByTestId("schedule-plan");
    const cfg = of("schedule_generate_preview")[0].args.config;
    expect(cfg.groups).toEqual({ groups: [[100, 101], [102, 103]], within: 2, across: 1, origin: "the shipped swedishschedule groups of test" });
    expect(cfg.odd_home).toBe("per_pair");
    expect(cfg.odd_hosts).toEqual([
      { home: 102, away: 100 },
      { home: 103, away: 100 },
      { home: 102, away: 101 },
      { home: 103, away: 101 },
    ]);
    expect(cfg.pair_quotas).toBeUndefined();
    // Changing a count clears the shipped origin (the table is now the user's) and invalidates the preview.
    fireEvent.change(screen.getByTestId("gen-within"), { target: { value: "4" } });
    expect(screen.queryByTestId("schedule-plan")).toBeNull();
  });

  it("generate: the blocker is shown; a preview sends the configured numbers and applies its review id", async () => {
    handlers.schedule_generation_options = () => ({
      members: [
        { team_id: 100n, slot: 0n, name: "Frölunda", abbrev: "FHC", games: 2, default: true },
        { team_id: 101n, slot: 1n, name: "Färjestad", abbrev: "FBK", games: 2, default: true },
      ],
      kept_rows: 0,
      rows_to_fill: 2,
      pairs: 1,
      suggested_rounds: 2,
      suggested_start_month: 8n,
      suggested_start_day: 15n,
      suggested_days_between_matchdays: 2,
      matchdays_per_round: 1,
      blocker: "2 games for 2 teams is fine here, but this text must be shown",
      notes: ["topology note"],
    });
    handlers.schedule_generate_preview = () => ({ change: plan({ ticket: "ws-a/90", rows_changed: 2, cells: 4 }), participants: 2, rounds: 2, matchdays: 2, generated_rows: 2, kept_rows: 0, games_per_participant: 2 });
    handlers.schedule_apply_review = (a) => ({ ...applied(a.ticket), kind: "generate", generation: { change: plan({}), participants: 2, rounds: 2, matchdays: 2, generated_rows: 2, kept_rows: 0, games_per_participant: 2 } });
    mount({ projectId: 7, leagueCode: 2 });
    await screen.findByTestId("schedule-summary");
    fireEvent.click(screen.getByText("Generate…"));
    await screen.findByTestId("generate-blocker");
    await waitFor(() => expect((screen.getByTestId("gen-rounds") as HTMLInputElement).value).toBe("2"));
    fireEvent.click(screen.getByTestId("gen-preview"));
    await screen.findByTestId("schedule-plan");
    expect(of("schedule_generate_preview")[0].args.config).toEqual({ participants: [100, 101], rounds: 2, start_month: 8, start_day: 15, days_between_matchdays: 2 });
    // Changing a number invalidates the plan; the old review is never applied.
    fireEvent.change(screen.getByTestId("gen-start-day"), { target: { value: "17" } });
    expect(screen.queryByTestId("schedule-plan")).toBeNull();
    expect((screen.getByTestId("gen-apply") as HTMLButtonElement).disabled).toBe(true);
    fireEvent.click(screen.getByTestId("gen-preview"));
    await screen.findByTestId("schedule-plan");
    expect(of("schedule_generate_preview")[1].args.config.start_day).toBe(16);
    fireEvent.click(screen.getByTestId("gen-apply"));
    await screen.findByTestId("schedule-notice");
    expect(of("schedule_apply_review").map((c) => c.args)).toEqual([{ projectId: 7, ticket: "ws-a/90" }]);
  });
});
