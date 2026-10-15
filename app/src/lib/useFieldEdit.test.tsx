// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider, useQuery } from "@tanstack/react-query";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
const mock = vi.hoisted(() => ({ call: vi.fn(), update: vi.fn() }));
vi.mock("./client", () => ({ call: mock.call }));
vi.mock("./project", () => ({ useProject: () => ({ update: mock.update }) }));
import { EditableField } from "../components/EditableField";
import { awaitFieldWrites } from "./fieldWrites";
import type { FieldViewDto } from "../bindings/FieldViewDto";

const initial = { studio_table: "ubPc", studio_field: "QbMR", name: "lastname", display: "Demitra", text: "Demitra",
  is_string: true, editable: true, enum_options: [], decoded: null, text_max_bytes: 30 } as unknown as FieldViewDto;
let saved: FieldViewDto;
let reads = vi.fn(async (): Promise<FieldViewDto> => saved);
let unrelated = vi.fn(async () => "portrait");
let finish: Array<{ args: any; resolve: () => void; reject: (e: Error) => void }>;
function App() {
  const field = useQuery({ queryKey: ["player", 1], queryFn: reads });
  useQuery({ queryKey: ["portrait", "same-file"], queryFn: unrelated });
  return field.data ? <EditableField projectId={1} record={281} field={field.data} label="Last Name" /> : null;
}
function mount() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false, staleTime: Infinity } } });
  render(<QueryClientProvider client={qc}><App /></QueryClientProvider>);
}
function edit(current: string, next: string) {
  fireEvent.click(screen.getByRole("button", { name: current }));
  fireEvent.change(screen.getByTestId("text-number-input"), { target: { value: next } });
  fireEvent.keyDown(screen.getByTestId("text-number-input"), { key: "Enter" });
}
beforeEach(() => {
  saved = initial;
  reads = vi.fn(async () => saved);
  unrelated = vi.fn(async () => "portrait");
  finish = [];
  mock.call.mockReset(); mock.update.mockReset();
  mock.call.mockImplementation((cmd, args) => cmd === "set_field" ? new Promise<void>((resolve, reject) => {
    finish.push({ args, resolve: () => { saved = { ...saved, text: args.textValue, display: args.textValue }; resolve(); }, reject });
  }) : Promise.resolve({ project_id: 1 }));
});
afterEach(cleanup);

it("shows each submitted value before IPC resolves, preserves write order, and combines refreshes", async () => {
  mount(); await screen.findByRole("button", { name: "Demitra" });
  edit("Demitra", "First");
  expect(screen.getByRole("button", { name: "First" })).toBeTruthy(); // No timer, network or waitFor.
  await waitFor(() => expect(finish).toHaveLength(1));
  edit("First", "Second");
  expect(screen.getByRole("button", { name: "Second" })).toBeTruthy();
  expect(finish).toHaveLength(1); // Second write is queued, not lost or sent ahead.
  let saveReady = false;
  const barrier = awaitFieldWrites(1).then(() => { saveReady = true; });
  await act(async () => { finish[0].resolve(); });
  await waitFor(() => expect(finish).toHaveLength(2));
  expect(saveReady).toBe(false);
  expect(screen.getByRole("button", { name: "Second" })).toBeTruthy();
  await act(async () => { finish[1].resolve(); await barrier; });
  await waitFor(() => expect(reads).toHaveBeenCalledTimes(2));
  expect(saveReady).toBe(true);
  expect(finish.map(f => f.args.textValue)).toEqual(["First", "Second"]);
  expect(unrelated).toHaveBeenCalledTimes(1);
  expect(mock.call.mock.calls.filter(([cmd]) => cmd === "project_summary")).toHaveLength(1);
  expect(screen.getByRole("button", { name: "Second" })).toBeTruthy();
});

it("rolls a rejected edit back and keeps its reason visible", async () => {
  mount(); await screen.findByRole("button", { name: "Demitra" });
  edit("Demitra", "Invalid");
  expect(screen.getByRole("button", { name: "Invalid" })).toBeTruthy();
  await waitFor(() => expect(finish).toHaveLength(1));
  await act(async () => { finish[0].reject(new Error("Rejected by validation")); });
  await screen.findByRole("button", { name: "Demitra" });
  expect(screen.getByText(/Rejected by validation/)).toBeTruthy();
  await waitFor(() => expect(reads).toHaveBeenCalledTimes(2));
});

it("rejecting a later edit preserves the preceding accepted value", async () => {
  mount(); await screen.findByRole("button", { name: "Demitra" });
  edit("Demitra", "Accepted");
  await waitFor(() => expect(finish).toHaveLength(1));
  edit("Accepted", "Rejected");
  await act(async () => { finish[0].resolve(); });
  await waitFor(() => expect(finish).toHaveLength(2));
  await act(async () => { finish[1].reject(new Error("Invalid later edit")); });
  await screen.findByRole("button", { name: "Accepted" });
  await waitFor(() => expect(reads).toHaveBeenCalledTimes(2));
  expect(screen.getByRole("button", { name: "Accepted" })).toBeTruthy();
});
