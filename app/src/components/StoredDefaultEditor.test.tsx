// @vitest-environment jsdom
//
// D3 — the stored-default editor: exactly what is dispatched for a preview and an apply (the
// ticket only), how a refusal / plan / stale-ticket error is shown, and that completions for a
// row or project that is no longer shown never repaint or push a foreign summary. Component
// evidence only; eligibility, bounds and the byte accounting are the Rust suite's job.
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { SettingRowDto } from "../bindings/SettingRowDto";
import type { SettingsEditPlanDto } from "../bindings/SettingsEditPlanDto";

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

import { StoredDefaultEditor } from "./StoredDefaultEditor";

function row(over: Partial<SettingRowDto> = {}): SettingRowDto {
  return {
    legacy_id: 72n,
    name: "PlayoffSeriesLength",
    label_key: "PlayoffSeriesLength",
    label_text: null,
    setting_id: 152n,
    siblings: [86n, 116n],
    is_choice: true,
    min: 0n,
    max: 4n,
    size: 5n,
    global: { code: 3n, label: "BestOf7", cells: ["stocksettings record 72 (legacyid 72) field default"], values: [3n], paired: false, consistent: null },
    context_defaults: [
      { context: 3n, context_name: "FROM_DYNASTY_FLOW", stored: { code: 3n, label: "BestOf7", cells: ["stocksettingdefaults record 152 (context 3, legacyid 72) field default"], values: [3n], paired: false, consistent: null }, choices: null, domain_note: null, enabled: null, visible: null, anomaly: null },
      { context: 30n, context_name: "FROM_BE_A_PRO_FLOW", stored: { code: 3n, label: "BestOf7", cells: ["stocksettingdefaults record 170 (context 30, legacyid 72) field default"], values: [3n], paired: false, consistent: null }, choices: null, domain_note: null, enabled: null, visible: null, anomaly: null },
    ],
    choices: [
      { code: 0n, key: "SingleKnockout", text: null },
      { code: 1n, key: "BestOf3", text: null },
      { code: 2n, key: "BestOf5", text: null },
      { code: 3n, key: "BestOf7", text: null },
      { code: 4n, key: "Authentic", text: null },
    ],
    label_source: "inferred",
    groups: [5n],
    dependents: [],
    eligibility: "eligible",
    role: "DYNASTY_PLAYOFFS_LENGTH",
    eligibility_reason: null,
    anomalies: [],
    ...over,
  };
}

function plan(over: Partial<SettingsEditPlanDto> = {}): SettingsEditPlanDto {
  return {
    kind: "database",
    ticket: "ws-a/1",
    legacy_id: 72n,
    context: 3n,
    setting_name: "PlayoffSeriesLength",
    label_text: null,
    role: "DYNASTY_PLAYOFFS_LENGTH",
    context_name: "FROM_DYNASTY_FLOW",
    table_name: "stocksettingdefaults",
    record: 152,
    before: 3n,
    before_label: "BestOf7",
    after: 2n,
    after_label: "BestOf5",
    max: 4n,
    label_source: "inferred",
    siblings: [86n, 116n],
    unchanged: [["global", 3n]],
    copy: "base 01.00",
    no_op: false,
    stock_anomalies: ["a"],
    description: "NHL 15 PS4 (base 01.00) — stored catalogue default of *PlayoffSeriesLength* for the FROM_DYNASTY_FLOW screen (context 3) changes from code 3 (*BestOf7*, label inferred) to code 2 (*BestOf5*, label inferred).",
    notes: ["Only the bits of the selected `default` field change"],
    paired: false,
    members: [],
    archive: null,
    ...over,
  };
}

let qc: QueryClient;
function mount(projectId = 7, setting: SettingRowDto = row()) {
  qc = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <StoredDefaultEditor projectId={projectId} setting={setting} />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  calls.length = 0;
  updates.length = 0;
  for (const k of Object.keys(handlers)) delete handlers[k];
});
afterEach(() => cleanup());

describe("StoredDefaultEditor", () => {
  it("previews the chosen cell and code, then applies by ticket only and adopts the summary", async () => {
    handlers.settings_default_preview = () => ({ plan: plan(), refusal_kind: null, refusal: null });
    handlers.settings_apply_review = () => ({ plan: plan(), outcome: { edits: 1, repairs: [] }, summary: { project_id: 7 } });
    mount();
    fireEvent.change(screen.getByTestId("target-select"), { target: { value: "3" } });
    fireEvent.change(screen.getByTestId("code-select"), { target: { value: "2" } });
    fireEvent.click(screen.getByTestId("preview-button"));
    await waitFor(() => expect(screen.getByTestId("edit-plan")).toBeTruthy());
    expect(calls).toEqual([{ cmd: "settings_default_preview", args: { projectId: 7, legacyId: 72n, context: 3n, code: 2n, installRoot: null } }]);
    expect(screen.getByTestId("edit-plan").textContent).toContain("changes from code 3");
    expect(screen.getByTestId("edit-plan").textContent).toContain("stocksettingdefaults record 152 · 3 → 2 · copy base 01.00");
    fireEvent.click(screen.getByTestId("apply-button"));
    await waitFor(() => expect(screen.getByTestId("edit-applied")).toBeTruthy());
    expect(calls[1]).toEqual({ cmd: "settings_apply_review", args: { projectId: 7, ticket: "ws-a/1" } });
    expect(updates).toEqual([{ project_id: 7 }]);
    expect(screen.queryByTestId("edit-plan")).toBeNull();
    expect(screen.getByTestId("edit-applied").textContent).toContain("Applied:");
  });

  it("shows a refusal as the server states it, and a stale-ticket apply error drops the held plan", async () => {
    handlers.settings_default_preview = (a: { context: bigint | null }) =>
      a.context === null
        ? { plan: plan({ context: null, table_name: "stocksettings", record: 72 }), refusal_kind: null, refusal: null }
        : { plan: null, refusal_kind: "no-row", refusal: "no stored default for PlayoffSeriesLength on screen 37; adding one is a table-growth operation that is not offered." };
    mount(7, row({ context_defaults: [...row().context_defaults, { context: 37n, context_name: "FROM_SEASON_MODE_FLOW", stored: { code: null, label: null, cells: [], values: [], paired: false, consistent: null }, choices: null, domain_note: null, enabled: null, visible: null, anomaly: null }] }));
    fireEvent.change(screen.getByTestId("target-select"), { target: { value: "37" } });
    fireEvent.change(screen.getByTestId("code-select"), { target: { value: "2" } });
    fireEvent.click(screen.getByTestId("preview-button"));
    await waitFor(() => expect(screen.getByTestId("edit-refusal")).toBeTruthy());
    expect(screen.getByTestId("edit-refusal").textContent).toContain("table-growth");
    expect(screen.getByTestId("edit-refusal").textContent).toContain("(no-row)");
    expect(screen.queryByTestId("edit-plan")).toBeNull();
    // Global cell: a plan; the apply is refused server-side (stale) → error shown, plan dropped.
    fireEvent.change(screen.getByTestId("target-select"), { target: { value: "global" } });
    fireEvent.click(screen.getByTestId("preview-button"));
    await waitFor(() => expect(screen.getByTestId("edit-plan")).toBeTruthy());
    expect(screen.queryByTestId("edit-refusal")).toBeNull();
    handlers.settings_apply_review = () => {
      throw new Error("PlayoffSeriesLength: the project changed since this preview (an edit, undo/redo, reload or another apply); preview again");
    };
    fireEvent.click(screen.getByTestId("apply-button"));
    await waitFor(() => expect(screen.getByTestId("edit-error")).toBeTruthy());
    expect(screen.getByTestId("edit-error").textContent).toContain("preview again");
    expect(screen.queryByTestId("edit-plan")).toBeNull();
    expect(updates).toEqual([]);
  });

  it("ignores a late preview for a row that is no longer shown and never pushes a foreign summary", async () => {
    const late = deferred<any>();
    handlers.settings_default_preview = (a: { legacyId: bigint }) => (a.legacyId === 72n ? late.promise : { plan: plan({ legacy_id: 68n, setting_name: "SeasonTieBreak", ticket: "ws-a/2", description: "SeasonTieBreak plan" }), refusal_kind: null, refusal: null });
    const view = mount();
    fireEvent.change(screen.getByTestId("code-select"), { target: { value: "2" } });
    fireEvent.click(screen.getByTestId("preview-button"));
    await waitFor(() => expect(calls.length).toBe(1));
    // The row changes while the preview is in flight.
    view.rerender(
      <QueryClientProvider client={qc}>
        <StoredDefaultEditor projectId={7} setting={row({ legacy_id: 68n, name: "SeasonTieBreak", choices: [{ code: 0n, key: "ContinuousOT", text: null }, { code: 2n, key: "5MinThenShootout", text: null }] })} />
      </QueryClientProvider>,
    );
    await act(async () => {
      late.resolve({ plan: plan(), refusal_kind: null, refusal: null });
      await late.promise;
    });
    expect(screen.queryByTestId("edit-plan")).toBeNull();
    // The new row previews its own plan.
    fireEvent.change(screen.getByTestId("code-select"), { target: { value: "0" } });
    fireEvent.click(screen.getByTestId("preview-button"));
    await waitFor(() => expect(screen.getByTestId("edit-plan")).toBeTruthy());
    expect(screen.getByTestId("edit-plan").textContent).toContain("SeasonTieBreak plan");
    expect(calls[1].args).toEqual({ projectId: 7, legacyId: 68n, context: null, code: 0n, installRoot: null });
    // A late apply completion for another project adopts nothing here.
    const lateApply = deferred<any>();
    handlers.settings_apply_review = () => lateApply.promise;
    fireEvent.click(screen.getByTestId("apply-button"));
    await waitFor(() => expect(calls.length).toBe(3));
    view.rerender(
      <QueryClientProvider client={qc}>
        <StoredDefaultEditor projectId={9} setting={row()} />
      </QueryClientProvider>,
    );
    await act(async () => {
      lateApply.resolve({ plan: plan({ legacy_id: 68n }), outcome: { edits: 1, repairs: [] }, summary: { project_id: 7 } });
      await lateApply.promise;
    });
    expect(updates).toEqual([]);
    expect(screen.queryByTestId("edit-applied")).toBeNull();
  });
});

describe("StoredDefaultEditor (CSV titles)", () => {
  it("previews with the install root, shows the paired members and reports the apply as pending", async () => {
    handlers.settings_default_preview = () => ({
      plan: plan({
        kind: "csv",
        ticket: "ws-a/9",
        context: null,
        table_name: "contextsetting.csv + setting.csv",
        record: 3,
        before: 4n,
        before_label: "Authentic",
        after: 3n,
        after_label: "BestOf7",
        copy: "",
        archive: "H:/Games/CUSA07569/databigs/inxp_00.big",
        paired: true,
        members: [
          { member: "contextsetting.csv", member_path: "nhl/data/common/gamemodes/setting_csv/contextsetting.csv", stored_form: "chunkzip", source_plain_sha256: "a", source_stored_len: 25805n, new_plain_sha256: "b", new_stored_len: 25805n, new_stored_sha256: "c", cells: [[3, "default_value", "4", "3"]], placement_prediction: "25805 bytes fit the member's slot (25805 + 3 slack)" },
          { member: "setting.csv", member_path: "nhl/data/common/gamemodes/setting_csv/setting.csv", stored_form: "chunkzip", source_plain_sha256: "d", source_stored_len: 7392n, new_plain_sha256: "e", new_stored_len: 7391n, new_stored_sha256: "f", cells: [[3, "default_value", "4", "3"]], placement_prediction: "7391 bytes fit the member's slot (7392 + 0 slack)" },
        ],
        description: "NHL18 PS4 — stored catalogue default of *DYNASTY_PLAYOFFS_LENGTH*: the global default stored in `setting.csv` row 72 **and** in `contextsetting.csv` row `_DEFAULT_`/72, both from code 4 to code 3.",
      }),
      refusal_kind: null,
      refusal: null,
    });
    handlers.settings_apply_review = () => ({ plan: plan({ kind: "csv", paired: true, description: "pending pair" }), outcome: { edits: 2, repairs: [] }, summary: { project_id: 7 } });
    qc = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } });
    render(
      <QueryClientProvider client={qc}>
        <StoredDefaultEditor projectId={7} setting={row({ global: { code: 4n, label: "Authentic", cells: ["x", "y"], values: [4n, 4n], paired: true, consistent: true } })} installRoot="H:/Games/CUSA07569" />
      </QueryClientProvider>,
    );
    fireEvent.change(screen.getByTestId("code-select"), { target: { value: "3" } });
    fireEvent.click(screen.getByTestId("preview-button"));
    await waitFor(() => expect(screen.getByTestId("edit-plan")).toBeTruthy());
    expect(calls[0].args).toEqual({ projectId: 7, legacyId: 72n, context: null, code: 3n, installRoot: "H:/Games/CUSA07569" });
    const members = screen.getByTestId("edit-plan-members").textContent ?? "";
    expect(members).toContain("contextsetting.csv (chunkzip): line 3 default_value 4→3 · 25805 → 25805 B stored");
    expect(members).toContain("setting.csv (chunkzip)");
    expect(screen.getByTestId("edit-plan").textContent).toContain("paired: two cells in two members, one operation");
    fireEvent.click(screen.getByTestId("apply-button"));
    await waitFor(() => expect(screen.getByTestId("edit-applied")).toBeTruthy());
    expect(screen.getByTestId("edit-applied").textContent).toContain("Pending (written by the next copied-target save)");
    expect(calls[1]).toEqual({ cmd: "settings_apply_review", args: { projectId: 7, ticket: "ws-a/9" } });
  });
});
