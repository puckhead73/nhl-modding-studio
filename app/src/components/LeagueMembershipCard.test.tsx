// @vitest-environment jsdom
//
// P1-C — the membership card on a PS4 title: choices come from the server with their mapping
// evidence, inferred / unknown rows are marked (never dressed up as names), a pool-only league
// offers its own id first, a single-group league with an unknown id says so, and a database no
// table describes falls back to plain numbers with the reason. Component evidence only (jsdom):
// validation and writes are the Rust suites' job; here we check what is displayed and dispatched.
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { MembershipOptionsDto } from "../bindings/MembershipOptionsDto";
import type { MembershipResultDto } from "../bindings/MembershipResultDto";

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

import { LeagueMembershipCard } from "./LeagueMembershipCard";

const choice = (value: number, label: string, evidence: string | null = "traced-static") => ({ value: BigInt(value), label, evidence });

/// NHL 21 PS4-shaped options for Boston: NHL structured (West → Pacific/Central, East →
/// Atlantic/Metropolitan, with NHL 15's two All-Star exceptions for the disclosure), AHL
/// structured (West → Central/Pacific, East → Atlantic/North, as the rows carry them), Spengler
/// pool-only (the NHL 18 flavour: one witnessed pool, inferred), Alumni single-group unknown 77.
function ps4Options(): MembershipOptionsDto {
  return {
    team_id: 1n,
    model: "global-groups",
    group_evidence: "traced",
    current: {
      league_code: 0n,
      league_label: "NHL®",
      league_group: choice(1, "NHL®"),
      conference: choice(7, "Eastern Conference"),
      division: choice(10, "Atlantic Division"),
    },
    leagues: [choice(0, "NHL®", null), choice(1, "AHL", null), choice(20, "Spengler Cup", null), choice(21, "TXT_LEAGUE_NHL_ALUMNI", null)],
    groups_by_league: [
      {
        league_code: 0n,
        league_group: 1n,
        league_group_evidence: "traced-static",
        conferences: [choice(6, "Western Conference"), choice(7, "Eastern Conference")],
        divisions: [choice(8, "Pacific Division"), choice(9, "Central Division"), choice(10, "Atlantic Division"), choice(11, "Metropolitan Division")],
        pairs: [[6n, 8n], [6n, 9n], [7n, 10n], [7n, 11n]],
        exceptional: [
          "Western Conference / Western Conference (a conference id carried in the division slot (All-Star / special squads); 1 row: team id 31)",
          "Eastern Conference / Eastern Conference (a conference id carried in the division slot (All-Star / special squads); 1 row: team id 30)",
        ],
      },
      {
        league_code: 1n,
        league_group: 12n,
        league_group_evidence: "traced-static",
        conferences: [choice(13, "Western Conference"), choice(14, "Eastern Conference")],
        divisions: [choice(15, "Atlantic Division"), choice(16, "North Division"), choice(17, "Central Division"), choice(18, "Pacific Division")],
        pairs: [[13n, 17n], [13n, 18n], [14n, 15n], [14n, 16n]],
        exceptional: [],
      },
      {
        league_code: 20n,
        league_group: 74n,
        league_group_evidence: "inferred",
        conferences: [choice(75, "TXT_GROUP_CATTINI", "inferred")],
        divisions: [choice(75, "TXT_GROUP_CATTINI", "inferred")],
        pairs: [[75n, 75n]],
        exceptional: [],
      },
      { league_code: 21n, league_group: 77n, league_group_evidence: "unknown", conferences: [], divisions: [], pairs: [], exceptional: [] },
    ],
    notes: [
      "conference and division are global group ids from this title's own table (traced; CUSA18182 eboot.bin group table at VA 0x5e041f8)",
      "group ids with no known name on this title (shown by number; a move into their leagues writes the id the rows already use): 77 — TXT_LEAGUE_NHL_ALUMNI / TXT_LEAGUE_NHL_ALL_TIME (research hypothesis, not a name: TXT_LEAGUE_NHL_ALUMNI)",
    ],
    blocker: null,
  };
}

function result(over: Partial<MembershipResultDto>): MembershipResultDto {
  return {
    team_id: 1n,
    table: "kOtt",
    record: 5,
    writes: [["xMjj", 1n], ["iIBl", 12n], ["OwTu", 14n], ["CGMK", 16n]],
    before: ps4Options().current,
    after: { league_code: 1n, league_label: "AHL", league_group: choice(12, "AHL"), conference: choice(14, "Eastern Conference"), division: choice(16, "North Division") },
    notes: [],
    outcome: { edits: 4, ok: true, findings: [] } as any,
    ...over,
  };
}

function mount(projectId = 7, teamId = 1) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  const ui = (p: number, t: number) => (
    <QueryClientProvider client={qc}>
      <LeagueMembershipCard projectId={p} teamId={t} />
    </QueryClientProvider>
  );
  const r = render(ui(projectId, teamId));
  return { ...r, switchTo: (p: number, t: number) => r.rerender(ui(p, t)) };
}

describe("LeagueMembershipCard on a PS4 title (P1-C)", () => {
  beforeEach(() => {
    calls.length = 0;
    updates.length = 0;
    for (const k of Object.keys(handlers)) delete handlers[k];
  });
  afterEach(cleanup);

  it("names the current PS4 conference and division from the title's table and offers traced choices", async () => {
    handlers.team_membership_options = () => ps4Options();
    mount();
    await screen.findByTestId("membership-current");
    expect(screen.getByTestId("membership-current").textContent).toContain("Eastern Conference · Atlantic Division");
    expect(screen.getByTestId("membership-current").textContent).toContain("global-groups · traced");
    expect(screen.queryByText("inferred")).toBeNull();
    // Pick the AHL: the conference select lists the AHL's own conferences; the division select
    // waits for a conference and then lists only the divisions paired with it (R1 F1).
    fireEvent.change(screen.getByLabelText("target league"), { target: { value: "1" } });
    const conf = screen.getByLabelText("conference") as HTMLSelectElement;
    expect(Array.from(conf.options).map((o) => o.text)).toEqual(["— choose —", "Western Conference", "Eastern Conference"]);
    const div = screen.getByLabelText("division") as HTMLSelectElement;
    expect(div.disabled).toBe(true);
    expect(Array.from(div.options).map((o) => o.text)).toEqual(["— choose a conference first —"]);
    fireEvent.change(conf, { target: { value: "13" } });
    expect(Array.from(div.options).map((o) => o.text)).toEqual(["— choose —", "Central Division", "Pacific Division"]);
    fireEvent.change(div, { target: { value: "17" } });
    // Switching the conference drops a division that is not paired with the new one.
    fireEvent.change(conf, { target: { value: "14" } });
    expect(Array.from(div.options).map((o) => o.text)).toEqual(["— choose —", "Atlantic Division", "North Division"]);
    expect(div.value).toBe("");
    expect((screen.getByText("Move team") as HTMLButtonElement).disabled).toBe(true);
    expect(screen.queryByTestId("membership-target-evidence")).toBeNull();
    expect(screen.queryByTestId("membership-exceptional")).toBeNull();
    // Move: exactly the chosen ids are dispatched, and the result names the resolved groups.
    handlers.set_team_membership = () => result({});
    fireEvent.change(div, { target: { value: "16" } });
    fireEvent.click(screen.getByText("Move team"));
    await screen.findByTestId("membership-result");
    const move = calls.find((c) => c.cmd === "set_team_membership")!;
    expect(move.args).toEqual({ projectId: 7, teamId: 1, leagueCode: 1, conference: 14, division: 16 });
    expect(screen.getByTestId("membership-result").textContent).toContain("→ AHL / Eastern Conference / North Division (4 cells, one undo step)");
  });

  it("marks inferred pools and offers only the witnessed pool pairing on a pool league", async () => {
    handlers.team_membership_options = () => ps4Options();
    mount();
    await screen.findByTestId("membership-current");
    fireEvent.change(screen.getByLabelText("target league"), { target: { value: "20" } });
    const conf = screen.getByLabelText("conference") as HTMLSelectElement;
    expect(Array.from(conf.options).map((o) => o.text)).toEqual(["— choose —", "TXT_GROUP_CATTINI (inferred)"]);
    expect(screen.getByTestId("membership-target-evidence").textContent).toContain("inferred from database usage");
    // Nothing chosen yet: the move stays disabled.
    expect((screen.getByText("Move team") as HTMLButtonElement).disabled).toBe(true);
    fireEvent.change(conf, { target: { value: "75" } });
    const div = screen.getByLabelText("division") as HTMLSelectElement;
    expect(Array.from(div.options).map((o) => o.text)).toEqual(["— choose —", "TXT_GROUP_CATTINI (inferred)"]);
    fireEvent.change(div, { target: { value: "75" } });
    expect((screen.getByText("Move team") as HTMLButtonElement).disabled).toBe(false);
  });

  it("discloses exceptional stock arrangements without offering them", async () => {
    handlers.team_membership_options = () => ps4Options();
    mount();
    await screen.findByTestId("membership-current");
    fireEvent.change(screen.getByLabelText("target league"), { target: { value: "0" } });
    fireEvent.change(screen.getByLabelText("conference"), { target: { value: "6" } });
    const div = screen.getByLabelText("division") as HTMLSelectElement;
    expect(Array.from(div.options).map((o) => o.text)).toEqual(["— choose —", "Pacific Division", "Central Division"]);
    const exc = screen.getByTestId("membership-exceptional");
    expect(exc.textContent).toContain("2 stock arrangements not offered");
    expect(exc.textContent).toContain("team id 31");
  });

  it("says when a single-group target's id has no known name, and never shows the hypothesis as the name", async () => {
    handlers.team_membership_options = () => ps4Options();
    mount();
    await screen.findByTestId("membership-current");
    fireEvent.change(screen.getByLabelText("target league"), { target: { value: "21" } });
    const single = screen.getByTestId("membership-single-group");
    expect(single.textContent).toContain("its own group id (77)");
    expect(single.textContent).toContain("no known name on this title");
    expect(screen.queryByLabelText("conference")).toBeNull();
    expect(screen.getByTestId("membership-notes").textContent).toContain("research hypothesis, not a name");
    // The league list shows the league's own key, not the hypothesis, for the target.
    const league = screen.getByLabelText("target league") as HTMLSelectElement;
    expect(league.options[league.selectedIndex].text).toBe("TXT_LEAGUE_NHL_ALUMNI");
    handlers.set_team_membership = () =>
      result({
        writes: [["xMjj", 21n], ["iIBl", 77n], ["OwTu", 77n], ["CGMK", 77n]],
        after: { league_code: 21n, league_label: "TXT_LEAGUE_NHL_ALUMNI", league_group: choice(77, "Unnamed group 77", "unknown"), conference: choice(77, "Unnamed group 77", "unknown"), division: choice(77, "Unnamed group 77", "unknown") },
      });
    fireEvent.click(screen.getByText("Move team"));
    await screen.findByTestId("membership-result");
    expect(calls.find((c) => c.cmd === "set_team_membership")!.args).toEqual({ projectId: 7, teamId: 1, leagueCode: 21, conference: null, division: null });
    expect(screen.getByTestId("membership-result").textContent).toContain("Unnamed group 77");
  });

  it("falls back to plain numbers with the reason when no table describes the database", async () => {
    handlers.team_membership_options = () => ({
      ...ps4Options(),
      model: "ps4-foreign-key",
      group_evidence: null,
      current: {
        league_code: 0n,
        league_label: "NHL®",
        league_group: choice(1, "league group 1 (no group table applies to this session)", null),
        conference: choice(7, "conference group 7 (no group table applies to this session)", null),
        division: choice(10, "division group 10 (no group table applies to this session)", null),
      },
      groups_by_league: [],
      notes: [
        "the league is a stockleagues row (a dangling code is refused); the row's group ids point into the game executable's table, and no baked table applies to this session, so they are written raw when given",
        "the nhl15 group table does not describe this database: 66 of 402 team rows carry their league's group from this table (180 name another league's group, 0 an id the table lacks); the database was opened as nhl15 but its team rows do not use that title's group ids, so they are shown raw",
      ],
    });
    mount();
    await screen.findByTestId("membership-current");
    expect(screen.getByTestId("membership-current").textContent).toContain("no group table applies to this session");
    expect(screen.getByLabelText("conference number")).toBeTruthy();
    expect(screen.getByTestId("membership-notes").textContent).toContain("does not describe this database");
    await waitFor(() => expect(calls.filter((c) => c.cmd === "team_membership_options")).toHaveLength(1));
  });

  // ---- R1 F3: completion ownership after the await on the project summary -------------------

  async function startMove(summary: Deferred<{ project_id: number }>) {
    handlers.team_membership_options = () => ps4Options();
    handlers.set_team_membership = () => result({});
    handlers.project_summary = (args: { projectId: number }) => summary.promise.then((s) => ({ ...s, project_id: args.projectId }));
    const m = mount(7, 1);
    await screen.findByTestId("membership-current");
    fireEvent.change(screen.getByLabelText("target league"), { target: { value: "1" } });
    fireEvent.change(screen.getByLabelText("conference"), { target: { value: "14" } });
    fireEvent.change(screen.getByLabelText("division"), { target: { value: "16" } });
    fireEvent.click(screen.getByText("Move team"));
    await screen.findByTestId("membership-result");
    await waitFor(() => expect(calls.some((c) => c.cmd === "project_summary")).toBe(true));
    return m;
  }

  it("applies the project summary and keeps the result when the same context completes", async () => {
    const summary = deferred<{ project_id: number }>();
    await startMove(summary);
    summary.resolve({ project_id: 7 });
    await waitFor(() => expect(updates).toEqual([{ project_id: 7 }]));
    expect(screen.getByTestId("membership-result").textContent).toContain("AHL");
  });

  it("never pushes an old project's summary or notice after the card switched project and team", async () => {
    const summary = deferred<{ project_id: number }>();
    const m = await startMove(summary);
    // The move completed for project 7 / team 1; its summary is still pending when the card moves on.
    m.switchTo(9, 2);
    await screen.findByTestId("membership-current");
    expect(screen.queryByTestId("membership-result")).toBeNull();
    summary.resolve({ project_id: 7 });
    await new Promise((r) => setTimeout(r, 20));
    expect(updates).toEqual([]);
    expect(screen.queryByTestId("membership-result")).toBeNull();
    expect(screen.queryByTestId("membership-summary-error")).toBeNull();
  });

  it("keeps the old project's summary out when only the team changed, and applies it when only the team changed on the same project", async () => {
    const summary = deferred<{ project_id: number }>();
    const m = await startMove(summary);
    m.switchTo(7, 2);
    await screen.findByTestId("membership-current");
    summary.resolve({ project_id: 7 });
    // Same project still open: the summary is that project's and is applied; the old team's
    // result is not shown on the new team's card.
    await waitFor(() => expect(updates).toEqual([{ project_id: 7 }]));
    expect(screen.queryByTestId("membership-result")).toBeNull();
  });

  it("shows a late summary error only in the context that started it, and never as a global update", async () => {
    const summary = deferred<{ project_id: number }>();
    await startMove(summary);
    summary.reject(new Error("summary offline"));
    await screen.findByTestId("membership-summary-error");
    expect(screen.getByTestId("membership-summary-error").textContent).toContain("summary offline");
    expect(updates).toEqual([]);
    expect(screen.getByTestId("membership-result").textContent).toContain("AHL");

    cleanup();
    calls.length = 0;
    updates.length = 0;
    const late = deferred<{ project_id: number }>();
    const m = await startMove(late);
    m.switchTo(9, 2);
    await screen.findByTestId("membership-current");
    late.reject(new Error("summary offline"));
    await new Promise((r) => setTimeout(r, 20));
    expect(screen.queryByTestId("membership-summary-error")).toBeNull();
    expect(updates).toEqual([]);
  });
});

