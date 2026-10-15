// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { MemoryRouter } from "react-router-dom";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
const mock = vi.hoisted(() => ({ call: vi.fn(), update: vi.fn() }));
vi.mock("../lib/client", () => ({ call: mock.call }));
vi.mock("../lib/project", () => ({ useProject: () => ({ project: { project_id: 1, tables: [] }, update: mock.update }) }));
import { LinesTab } from "./LinesTab";
import { JerseyCaptaincyTab } from "./JerseyCaptaincyTab";
const entries = [
  { slot_record: 10, player_id: 100, name: "First player", position_label: "C", jersey: 10, captaincy: 1, dressed: true, slots: ["abcd"], style: null },
  { slot_record: 11, player_id: 101, name: "Second player", position_label: "C", jersey: 11, captaincy: 0, dressed: true, slots: [], style: null },
];
beforeEach(() => {
  mock.call.mockImplementation(async command => command === "lines_view" ? { team_id: 14, slot_fields: [{ studio_field: "abcd", name: "l1c" }], entries } : command === "list_players" ? entries.map((e, i) => ({ player_id: e.player_id, overall: 80 + i })) : {});
});
afterEach(() => { cleanup(); vi.clearAllMocks(); });
function mount(lines: boolean) {
  render(<MemoryRouter><QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>{lines ? <LinesTab projectId={1} teamId={14} /> : <JerseyCaptaincyTab projectId={1} teamId={14} retiredNumbers={[]} />}</QueryClientProvider></MemoryRouter>);
}
it("submits Best lines as one roster operation", async () => {
  mount(true);
  fireEvent.click(await screen.findByRole("button", { name: "Best lines" }));
  await waitFor(() => expect(mock.call).toHaveBeenCalledWith("set_team_roster", { projectId: 1, team: 14, writes: [[10, "abcd", 0], [11, "abcd", 1]] }));
  expect(mock.call.mock.calls.some(([cmd]) => cmd === "set_field")).toBe(false);
});
it("scratches through one backend operation without clearing individual slots first", async () => {
  mount(true);
  const controls = await screen.findAllByRole("combobox", { name: "Lineup status" });
  fireEvent.change(controls[0], { target: { value: "scratched" } });
  await waitFor(() => expect(mock.call).toHaveBeenCalledWith("set_dressed", { projectId: 1, team: 14, player: 101, dressed: false }));
  expect(mock.call.mock.calls.some(([cmd]) => cmd === "set_field")).toBe(false);
});
it("replaces captain in one operation and hides unsupported retired-number controls", async () => {
  mount(false);
  const captain = await screen.findByRole("combobox", { name: /^Captain/ });
  expect(screen.queryByRole("checkbox")).toBeNull();
  fireEvent.change(captain, { target: { value: "101" } });
  await waitFor(() => expect(mock.call).toHaveBeenCalledWith("set_team_roster", { projectId: 1, team: 14, writes: [[10, "mCcl", 0], [11, "mCcl", 1]] }));
  expect(mock.call.mock.calls.some(([cmd]) => cmd === "set_captaincy")).toBe(false);
});

function transfer() {
  const data = new Map<string, string>();
  return { effectAllowed: "", dropEffect: "", setData: (type: string, value: string) => data.set(type, value), getData: (type: string) => data.get(type) ?? "" };
}
it("accepts a roster-card drag and renders the placement before the write finishes", async () => {
  let finish!: (v: unknown) => void;
  const base = mock.call.getMockImplementation()!;
  mock.call.mockImplementation((cmd, args) => cmd === "set_team_roster" ? new Promise(resolve => { finish = resolve; }) : base(cmd, args));
  mount(true);
  await screen.findByRole("button", { name: "Best lines" });
  const source = screen.getAllByText("Second player").map(e => e.closest('[draggable="true"]')).find(Boolean)!;
  const target = screen.getAllByText("First player").map(e => e.closest('[draggable="true"]')).find(Boolean)!;
  const dataTransfer = transfer();
  fireEvent.dragStart(source, { dataTransfer });
  expect(fireEvent.dragOver(target, { dataTransfer })).toBe(false);
  fireEvent.drop(target, { dataTransfer });
  await waitFor(() => expect(mock.call).toHaveBeenCalledWith("set_team_roster", { projectId: 1, team: 14, writes: [[10, "abcd", 0], [11, "abcd", 1]] }));
  expect(target.textContent).toContain("Second player");
  finish({});
});
it("shows captaincy immediately and rolls back a rejected change", async () => {
  let reject!: (e: Error) => void;
  const base = mock.call.getMockImplementation()!;
  mock.call.mockImplementation((cmd, args) => cmd === "set_team_roster" ? new Promise((_resolve, fail) => { reject = fail; }) : base(cmd, args));
  mount(false);
  const captain = await screen.findByRole("combobox", { name: /^Captain/ }) as HTMLSelectElement;
  fireEvent.change(captain, { target: { value: "101" } });
  await waitFor(() => expect(captain.value).toBe("101"));
  reject(new Error("Player cannot be assigned"));
  await screen.findByText(/Player cannot be assigned/);
  expect(captain.value).toBe("100");
});
it("accepts a number drag and displays its pending assignment immediately", async () => {
  let finish!: (v: unknown) => void;
  const base = mock.call.getMockImplementation()!;
  mock.call.mockImplementation((cmd, args) => cmd === "set_jersey_number" ? new Promise(resolve => { finish = resolve; }) : base(cmd, args));
  mount(false);
  const source = await screen.findByTitle("#90");
  const target = screen.getByText("First player").closest("li")!.querySelector("div")!;
  const dataTransfer = transfer();
  fireEvent.dragStart(source, { dataTransfer });
  fireEvent.dragOver(target, { dataTransfer });
  fireEvent.drop(target, { dataTransfer });
  await waitFor(() => expect(mock.call).toHaveBeenCalledWith("set_jersey_number", { projectId: 1, team: 14, player: 100, number: 90 }));
  expect(target.textContent).toContain("#90");
  finish({});
});

it("the Windows webview leaves HTML drag-and-drop to the editor", async () => {
  const { default: config } = await import("../../../src-tauri/tauri.conf.json");
  expect(config.app.windows.find((w: { label: string }) => w.label === "main")?.dragDropEnabled).toBe(false);
});
