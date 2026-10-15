// @vitest-environment jsdom
//
// Teams 2.2 (overnight 2026-09-24, package T1) — the home-arena cell is a disclosed two-step
// assignment: picking an arena writes nothing; the backend preview (who already plays at the
// chosen arena, who stays at the current one or that it becomes vacant, or the refusal) is shown
// first; only Assign dispatches `set_field`, on the previewed cell with the previewed arena id.
// Pending / failed previews never dispatch; a refusal disables Assign; a change of candidate
// re-previews; a target change (team / project) drops the candidate. Component evidence only.
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { MemoryRouter } from "react-router-dom";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { FieldViewDto } from "../bindings/FieldViewDto";
import type { ArenaAssignmentPreviewDto } from "../bindings/ArenaAssignmentPreviewDto";

const calls: Array<{ cmd: string; args: any }> = [];
const handlers: Record<string, (args: any) => unknown> = {};
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
vi.mock("../lib/client", () => ({
  call: (cmd: string, args: any) => {
    calls.push({ cmd, args });
    const h = handlers[cmd];
    if (!h) return Promise.reject(new Error(`unmocked ${cmd}`));
    return Promise.resolve().then(() => h(args));
  },
}));
const updates: unknown[] = [];
vi.mock("../lib/project", () => ({
  useProject: () => ({ update: (s: unknown) => updates.push(s) }),
}));

import { ArenaSelect } from "./EditableField";

const FIELD = { studio_table: "kOtt", studio_field: "arenaid", name: "arenaid", kind: "int", decoded: 5, display: "5", editable: true } as unknown as FieldViewDto;
const ARENAS = [
  { arena_id: 5, record: 0, name: "Honda Center", city: "Anaheim, CA", team_count: 1 },
  { arena_id: 7, record: 1, name: "Rogers Arena", city: "Vancouver, BC", team_count: 1 },
  { arena_id: 9, record: 2, name: "EA SPORTS Arena", city: "", team_count: 32 },
];
function previewOf(over: Partial<ArenaAssignmentPreviewDto> = {}): ArenaAssignmentPreviewDto {
  return {
    team_id: 1,
    team_name: "Anaheim Ducks",
    table: "kOtt",
    record: 3,
    field: "arenaid",
    from: { arena_id: 5, name: "Honda Center", city: "Anaheim, CA" },
    from_raw: 5,
    from_other_tenants: [],
    from_becomes_vacant: true,
    to: { arena_id: 7, name: "Rogers Arena", city: "Vancouver, BC" },
    to_raw: 7,
    to_tenants: ["Vancouver Canucks"],
    allowed: true,
    refusal: null,
    ...over,
  };
}

function tree(qc: QueryClient, props: { projectId?: number; teamId?: number; record?: number } = {}) {
  return (
    <QueryClientProvider client={qc}>
      <MemoryRouter>
        <ArenaSelect projectId={props.projectId ?? 7} teamId={props.teamId ?? 1} record={props.record ?? 3} field={FIELD} arena={{ arena_id: 5, name: "Honda Center", city: "Anaheim, CA" }} />
      </MemoryRouter>
    </QueryClientProvider>
  );
}
function mount(props?: { projectId?: number; teamId?: number; record?: number }) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } });
  const r = render(tree(qc, props));
  return { ...r, retarget: (n: { projectId?: number; teamId?: number; record?: number }) => r.rerender(tree(qc, n)) };
}
const writes = () => calls.filter((c) => c.cmd === "set_field");
const previews = () => calls.filter((c) => c.cmd === "arena_assignment_preview");
async function openAndPick(value: string) {
  fireEvent.click(screen.getByTestId("arena-change"));
  await screen.findByTestId("arena-select");
  await waitFor(() => expect(screen.getAllByRole("option").length).toBeGreaterThan(1));
  fireEvent.change(screen.getByTestId("arena-select"), { target: { value } });
}

beforeEach(() => {
  calls.length = 0;
  updates.length = 0;
  for (const k of Object.keys(handlers)) delete handlers[k];
  handlers.list_arenas = () => ARENAS;
  handlers.project_summary = (a: any) => ({ project_id: a.projectId, dirty: true });
  handlers.set_field = () => ({ edits: 1, repairs: [] });
  handlers.arena_assignment_preview = () => previewOf();
});
afterEach(cleanup);

describe("ArenaSelect (disclosed assignment)", () => {
  it("picking an arena writes nothing, shows who plays there and who stays, and Assign writes the previewed cell once", async () => {
    mount();
    await openAndPick("7");
    await screen.findByTestId("arena-disclosure-to");
    expect(writes()).toHaveLength(0);
    expect(previews()[0].args).toEqual({ projectId: 7, teamId: 1, toArena: 7 });
    expect(screen.getByTestId("arena-disclosure-to").textContent).toMatch(/1 team already plays there: Vancouver Canucks/);
    expect(screen.getByTestId("arena-disclosure-from").textContent).toMatch(/Honda Center — Anaheim, CA will have no home team/);
    fireEvent.click(screen.getByTestId("arena-assign"));
    fireEvent.click(screen.getByTestId("arena-assign"));
    await waitFor(() => expect(updates).toHaveLength(1));
    expect(writes()).toHaveLength(1);
    expect(writes()[0].args).toEqual({ projectId: 7, table: "kOtt", record: 3, field: "arenaid", intValue: 7 });
    await waitFor(() => expect(screen.queryByTestId("arena-select")).toBeNull());
  });

  it("shows who stays behind when the old arena keeps tenants", async () => {
    handlers.arena_assignment_preview = () => previewOf({ from_other_tenants: ["Anaheim Ducks Alumni", "Ducks Prospects"], from_becomes_vacant: false, to_tenants: [] });
    mount();
    await openAndPick("7");
    await screen.findByTestId("arena-disclosure-from");
    expect(screen.getByTestId("arena-disclosure-to").textContent).toBe("No other team plays there.");
    expect(screen.getByTestId("arena-disclosure-from").textContent).toMatch(/keeps Anaheim Ducks Alumni, Ducks Prospects/);
  });

  it("a refused preview disables Assign and never dispatches", async () => {
    handlers.arena_assignment_preview = () => previewOf({ allowed: false, refusal: "Anaheim Ducks already plays at Honda Center; nothing would change", to_raw: 5 });
    mount();
    await openAndPick("5");
    await screen.findByTestId("arena-disclosure-refusal");
    expect((screen.getByTestId("arena-assign") as HTMLButtonElement).disabled).toBe(true);
    fireEvent.click(screen.getByTestId("arena-assign"));
    expect(writes()).toHaveLength(0);
    // Cancel closes the picker.
    fireEvent.click(screen.getByTestId("arena-cancel"));
    await waitFor(() => expect(screen.queryByTestId("arena-select")).toBeNull());
  });

  it("a pending or failed preview never dispatches; a new candidate re-previews", async () => {
    const held = deferred<ArenaAssignmentPreviewDto>();
    let n = 0;
    handlers.arena_assignment_preview = (a: any) => {
      n += 1;
      if (n === 1) return held.promise;
      if (n === 2) return Promise.reject(new Error("could not read"));
      return previewOf({ to: { arena_id: Number(a.toArena), name: "EA SPORTS Arena", city: "" }, to_raw: a.toArena, to_tenants: ["A", "B"] });
    };
    mount();
    await openAndPick("7");
    await screen.findByTestId("arena-disclosure-loading");
    expect((screen.getByTestId("arena-assign") as HTMLButtonElement).disabled).toBe(true);
    fireEvent.click(screen.getByTestId("arena-assign"));
    expect(writes()).toHaveLength(0);
    // Switch the candidate before the first answers: the second preview fails.
    fireEvent.change(screen.getByTestId("arena-select"), { target: { value: "9" } });
    await screen.findByTestId("arena-disclosure-error");
    expect((screen.getByTestId("arena-assign") as HTMLButtonElement).disabled).toBe(true);
    held.resolve(previewOf());
    await new Promise((r) => setTimeout(r, 0));
    // The stale first answer never shows: the current candidate is 9 and its preview failed.
    expect(screen.queryByTestId("arena-disclosure-to")).toBeNull();
    // Picking again re-previews with the new candidate.
    fireEvent.change(screen.getByTestId("arena-select"), { target: { value: "7" } });
    await screen.findByTestId("arena-disclosure-to");
    expect(screen.getByTestId("arena-disclosure-to").textContent).toMatch(/2 teams already play there: A, B/);
    expect(previews().map((c) => c.args.toArena)).toEqual([7, 9, 7]);
    expect(writes()).toHaveLength(0);
  });

  it("a target change (another team) drops the candidate and its disclosure", async () => {
    const v = mount();
    await openAndPick("7");
    await screen.findByTestId("arena-disclosure-to");
    v.retarget({ teamId: 2 });
    await waitFor(() => expect(screen.queryByTestId("arena-disclosure")).toBeNull());
    expect(screen.queryByTestId("arena-select")).toBeNull();
    expect(writes()).toHaveLength(0);
  });

  it("a failed write shows the error and keeps the disclosure for another attempt", async () => {
    handlers.set_field = () => Promise.reject(new Error("refused: out of range"));
    mount();
    await openAndPick("7");
    await screen.findByTestId("arena-disclosure-to");
    fireEvent.click(screen.getByTestId("arena-assign"));
    await screen.findByTestId("arena-assign-error");
    expect(screen.getByTestId("arena-assign-error").textContent).toMatch(/out of range/);
    expect(screen.getByTestId("arena-disclosure-to")).toBeTruthy();
    expect(updates).toHaveLength(0);
  });
});
