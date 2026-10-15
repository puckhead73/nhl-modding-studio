// @vitest-environment jsdom
//
// D2 — the read-only settings catalogue page: what is displayed for a catalogue result, how a
// blocker is shown, that the request is bound to the project + install root, and that a late
// response for a project that is no longer open never repaints the current one. Component
// evidence only (jsdom); the decoding and joins are the Rust suites' job.
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { SettingsCatalogueResultDto } from "../bindings/SettingsCatalogueResultDto";
import type { SettingRowDto } from "../bindings/SettingRowDto";

type Deferred<T> = { promise: Promise<T>; resolve: (v: T) => void };
function deferred<T>(): Deferred<T> {
  let resolve!: (v: T) => void;
  const promise = new Promise<T>((res) => {
    resolve = res;
  });
  return { promise, resolve };
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
let projectState: { project: { project_id: number }; install: { install_dir: string } | null; iso: null } = {
  project: { project_id: 7 },
  install: { install_dir: "H:\\Games\\CUSA07569" },
  iso: null,
};
vi.mock("../lib/project", () => ({
  useProject: () => projectState,
}));

import { GameSettingsPage } from "./GameSettings";

function row(over: Partial<SettingRowDto>): SettingRowDto {
  return {
    legacy_id: 72n,
    name: "DYNASTY_PLAYOFFS_LENGTH",
    label_key: "PlayoffSeriesLength",
    label_text: null,
    setting_id: null,
    siblings: [],
    is_choice: true,
    min: 0n,
    max: 4n,
    size: 5n,
    global: { code: 4n, label: "Authentic", cells: ["setting.csv line 3 column default_value", "contextsetting.csv line 3 (_DEFAULT_ context -2, setting 72) column default_value"], values: [4n, 4n], paired: true, consistent: true },
    context_defaults: [
      { context: 3n, context_name: "DISPLAY_FRANCHISE_CREATION_CLASIC", stored: { code: 4n, label: "Authentic", cells: ["contextsetting.csv line 9 (context 3, setting 72) column default_value"], values: [4n], paired: false, consistent: null }, choices: null, domain_note: null, enabled: true, visible: true, anomaly: null },
    ],
    choices: [
      { code: 0n, key: "SingleKnockout", text: null },
      { code: 1n, key: "BestOf3", text: null },
      { code: 2n, key: "BestOf5", text: null },
      { code: 3n, key: "BestOf7", text: null },
      { code: 4n, key: "Authentic", text: null },
    ],
    label_source: "explicit",
    groups: [5n],
    dependents: [],
    eligibility: "eligible",
    role: "DYNASTY_PLAYOFFS_LENGTH",
    eligibility_reason: null,
    anomalies: [],
    ...over,
  };
}

function catalogue(settings: SettingRowDto[], over: Partial<NonNullable<SettingsCatalogueResultDto["catalogue"]>> = {}): SettingsCatalogueResultDto {
  return {
    catalogue: {
      game: "nhl18",
      platform: "ps4",
      source: { kind: "setting-csv", copy: null, database_sha256: null, tables: [], origin: "H:\\Games\\CUSA07569\\databigs\\inxp_00.big", variant: "nhl18-21", members: [{ path: "nhl/data/common/gamemodes/setting_csv/setting.csv", plain_len: 31518n, plain_sha256: "ab", stored_form: "chunkzip" }] },
      contexts: [
        { id: -2n, name: "_DEFAULT_", name_source: "explicit", defaults_rows: 300, layout_rows: 0, reserved: true },
        { id: 3n, name: "DISPLAY_FRANCHISE_CREATION_CLASIC", name_source: "explicit", defaults_rows: 40, layout_rows: 3, reserved: false },
      ],
      groups: [{ id: 5n, name: "QUICK_DEFAULT_GM", name_source: "explicit", parent: null, members: 12 }],
      settings,
      absent_eligible: [83n, 89n, 118n, 119n, 123n],
      anomalies: ["contextsetting.csv: 2 row(s) store an empty default (null; never offered as a target)"],
      notes: ["Whether a creation screen offers a stored default is not established."],
      dictionary: null,
      ...over,
    },
    blocker_kind: null,
    blocker: null,
  };
}

let qc: QueryClient;
function mount() {
  qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <GameSettingsPage />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  calls.length = 0;
  for (const k of Object.keys(handlers)) delete handlers[k];
  projectState = { project: { project_id: 7 }, install: { install_dir: "H:\\Games\\CUSA07569" }, iso: null };
});
afterEach(() => cleanup());

describe("GameSettingsPage", () => {
  it("requests the catalogue for the open project and its install root, and lists the contract rows first", async () => {
    handlers.settings_catalogue = () => catalogue([row({}), row({ legacy_id: 0n, name: "SKILL_LEVEL", role: null, eligibility: "out-of-scope", global: { code: 1n, label: "Pro", cells: ["setting.csv line 2 column default_value"], values: [1n, 1n], paired: true, consistent: true } }), row({ legacy_id: 82n, name: "TOURNAMENT_NUM_TEAMS", role: null, eligibility: "excluded", eligibility_reason: "bounds differ between copies" })]);
    mount();
    await waitFor(() => expect(screen.getByTestId("settings-source")).toBeTruthy());
    expect(calls).toEqual([{ cmd: "settings_catalogue", args: { projectId: 7, installRoot: "H:\\Games\\CUSA07569" } }]);
    expect(screen.getByTestId("settings-source").textContent).toContain("NHL 18 · PS4");
    expect(screen.getByTestId("settings-source").textContent).toContain("schema nhl18-21");
    // Contract scope by default: the eligible and excluded rows, not the out-of-scope one.
    expect(screen.getByTestId("setting-row-72")).toBeTruthy();
    expect(screen.getByTestId("setting-row-82")).toBeTruthy();
    expect(screen.queryByTestId("setting-row-0")).toBeNull();
    expect(screen.getByTestId("setting-row-72").textContent).toContain("stored twice, equal");
    expect(screen.getByTestId("setting-row-72").textContent).toContain("in contract");
    expect(screen.getByTestId("setting-row-82").textContent).toContain("excluded");
    expect(screen.getByText(/5 contract ids absent on this title/)).toBeTruthy();
    // "all" shows the out-of-scope row too.
    fireEvent.click(screen.getByLabelText(/all 3 settings/));
    expect(screen.getByTestId("setting-row-0")).toBeTruthy();
  });

  it("shows the selected row's stored defaults, screens, cells and codes, and the disclosures", async () => {
    handlers.settings_catalogue = () => catalogue([row({}), row({ legacy_id: 120n, name: "GAME_MODE_HOME_AND_AWAY_SCHEDULE", role: "GAME_MODE_HOME_AND_AWAY_SCHEDULE", global: { code: 1n, label: "ON", cells: ["a", "b"], values: [1n, 0n], paired: true, consistent: false }, anomalies: ["the two stored global copies disagree: setting.csv Some(1) vs _DEFAULT_ row Some(0)"] })]);
    mount();
    await waitFor(() => expect(screen.getByTestId("setting-row-72")).toBeTruthy());
    expect(screen.queryByTestId("setting-detail")).toBeNull();
    fireEvent.click(screen.getByTestId("setting-row-72"));
    const detail = screen.getByTestId("setting-detail");
    expect(screen.getByTestId("stored-default-editor")).toBeTruthy();
    expect(detail.textContent).toContain("rewrites one cell of a CSV text member");
    expect(detail.textContent).toContain("setting.csv line 3 column default_value");
    expect(detail.textContent).toContain("contextsetting.csv line 3 (_DEFAULT_ context -2, setting 72)");
    const ctx = screen.getByTestId("context-default-3");
    expect(ctx.textContent).toContain("DISPLAY_FRANCHISE_CREATION_CLASIC");
    expect(ctx.textContent).toContain("Authentic");
    expect(ctx.textContent).toContain("contextsetting.csv line 9 (context 3, setting 72)");
    expect(detail.textContent).toContain("labels explicit");
    expect(detail.textContent).toContain("SingleKnockout");
    expect(detail.textContent).toContain("QUICK_DEFAULT_GM");
    // An inconsistent pair is shown with both values, never collapsed.
    fireEvent.click(screen.getByTestId("setting-row-120"));
    expect(screen.getByTestId("setting-row-120").textContent).toContain("stored twice, differ: 1 / 0");
    expect(screen.getByTestId("setting-detail").textContent).toContain("copies disagree");
    // Disclosures.
    fireEvent.click(screen.getByTestId("settings-disclosures"));
    expect(screen.getByText(/not established/)).toBeTruthy();
    expect(screen.getByText(/store an empty default/)).toBeTruthy();
  });

  it("marks lineage names and inferred labels on an NHL 15 catalogue", async () => {
    handlers.settings_catalogue = () =>
      catalogue([row({ name: "PlayoffSeriesLength", setting_id: 152n, siblings: [86n, 116n], label_source: "inferred", global: { code: 3n, label: "BestOf7", cells: ["stocksettings record 72 (legacyid 72) field default"], values: [3n], paired: false, consistent: null }, context_defaults: [{ context: 3n, context_name: "FROM_DYNASTY_FLOW", stored: { code: 3n, label: "BestOf7", cells: ["stocksettingdefaults record 40 (context 3, legacyid 72) field default"], values: [3n], paired: false, consistent: null }, choices: null, domain_note: null, enabled: null, visible: null, anomaly: null }] })], {
        game: "nhl15",
        source: { kind: "database", copy: "base 01.00", database_sha256: "ae9d", tables: [{ name: "stocksettings", raw_id: "fRvu", rows: 223, max_records: 223 }], origin: null, variant: null, members: [] },
        contexts: [{ id: 3n, name: "FROM_DYNASTY_FLOW", name_source: "lineage", defaults_rows: 26, layout_rows: 3, reserved: false }],
        absent_eligible: [],
      });
    mount();
    await waitFor(() => expect(screen.getByTestId("setting-row-72")).toBeTruthy());
    expect(screen.getByTestId("settings-source").textContent).toContain("base 01.00");
    expect(screen.getByTestId("setting-row-72").textContent).toContain("inferred");
    fireEvent.click(screen.getByTestId("setting-row-72"));
    expect(screen.getByText(/shared with rows 86, 116/)).toBeTruthy();
    expect(screen.getByTestId("context-default-3").textContent).toContain("lineage");
    expect(screen.queryByText(/contract ids absent/)).toBeNull();
  });

  it("shows the blocker for a target without a catalogue", async () => {
    handlers.settings_catalogue = () => ({ catalogue: null, blocker_kind: "unsupported-title", blocker: "nhllegacy on Xbox 360: the setting definitions live only in a compiled table of the executable" });
    projectState = { project: { project_id: 9 }, install: null, iso: null };
    mount();
    await waitFor(() => expect(screen.getByTestId("settings-blocker")).toBeTruthy());
    expect(calls).toEqual([{ cmd: "settings_catalogue", args: { projectId: 9, installRoot: null } }]);
    expect(screen.getByTestId("settings-blocker").textContent).toContain("compiled table of the executable");
    expect(screen.getByTestId("settings-blocker").textContent).toContain("unsupported-title");
  });

  it("drops a late response for a project that is no longer open and keeps the selection per project", async () => {
    const late = deferred<SettingsCatalogueResultDto>();
    let n = 0;
    handlers.settings_catalogue = (args: { projectId: number }) => {
      n += 1;
      if (args.projectId === 7) return late.promise;
      return catalogue([row({ legacy_id: 68n, name: "DYNASTY_SEASON_TIE_BREAK", role: "DYNASTY_SEASON_TIE_BREAK" })], { game: "nhl21" });
    };
    const view = mount();
    await waitFor(() => expect(n).toBe(1));
    // The project is replaced while the first request is in flight.
    projectState = { project: { project_id: 8 }, install: { install_dir: "H:\\Games\\CUSA18182" }, iso: null };
    view.rerender(
      <QueryClientProvider client={qc}>
        <GameSettingsPage />
      </QueryClientProvider>,
    );
    await waitFor(() => expect(screen.getByTestId("setting-row-68")).toBeTruthy());
    expect(calls.map((c) => c.args)).toEqual([
      { projectId: 7, installRoot: "H:\\Games\\CUSA07569" },
      { projectId: 8, installRoot: "H:\\Games\\CUSA18182" },
    ]);
    fireEvent.click(screen.getByTestId("setting-row-68"));
    // The detail card's title names the selected row (the list row is the other match).
    expect(screen.getAllByText(/DYNASTY_SEASON_TIE_BREAK/).length).toBeGreaterThanOrEqual(2);
    expect(screen.getByTestId("setting-detail")).toBeTruthy();
    // The old project's response arrives now: it must not repaint the NHL 21 view.
    await act(async () => {
      late.resolve(catalogue([row({})]));
      await late.promise;
    });
    expect(screen.getByTestId("settings-source").textContent).toContain("NHL 21");
    expect(screen.queryByTestId("setting-row-72")).toBeNull();
    expect(screen.getAllByText(/DYNASTY_SEASON_TIE_BREAK/).length).toBeGreaterThanOrEqual(2);
    // Switching back to project 7 re-uses its (now cached) result and starts with no selection.
    projectState = { project: { project_id: 7 }, install: { install_dir: "H:\\Games\\CUSA07569" }, iso: null };
    view.rerender(
      <QueryClientProvider client={qc}>
        <GameSettingsPage />
      </QueryClientProvider>,
    );
    await waitFor(() => expect(screen.getByTestId("setting-row-72")).toBeTruthy());
    expect(screen.queryByTestId("setting-detail")).toBeNull();
  });
});
