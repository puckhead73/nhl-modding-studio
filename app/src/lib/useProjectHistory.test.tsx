// @vitest-environment jsdom
import { act, cleanup, fireEvent, renderHook, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { ReactNode } from "react";
const mock = vi.hoisted(() => ({ call: vi.fn(), update: vi.fn() }));
vi.mock("./client", () => ({ call: mock.call }));
vi.mock("./project", () => ({ useProject: () => ({ project: { project_id: 1 }, update: mock.update }) }));
import { useProjectHistory } from "./useProjectHistory";
import { queueFieldWrite } from "./fieldWrites";
beforeEach(() => { mock.call.mockImplementation(async command => command === "project_summary" ? { project_id: 1, undo_depth: 0, redo_depth: 1 } : true); });
afterEach(() => { cleanup(); vi.clearAllMocks(); document.body.innerHTML = ""; });
function mount(blocked = false) {
  const qc = new QueryClient();
  const invalidate = vi.spyOn(qc, "invalidateQueries");
  const hook = renderHook(() => useProjectHistory(blocked), { wrapper: ({ children }: { children: ReactNode }) => <QueryClientProvider client={qc}>{children}</QueryClientProvider> });
  return { ...hook, invalidate };
}
it.each([
  [{ key: "z", ctrlKey: true }, "undo"],
  [{ key: "y", ctrlKey: true }, "redo"],
  [{ key: "Z", ctrlKey: true, shiftKey: true }, "redo"],
  [{ key: "z", metaKey: true }, "undo"],
] as const)("handles %j and refreshes the project", async (keys, command) => {
  const { invalidate } = mount();
  fireEvent.keyDown(window, keys);
  await waitFor(() => expect(mock.update).toHaveBeenCalled());
  expect(mock.call).toHaveBeenNthCalledWith(1, command, { projectId: 1 });
  expect(invalidate).toHaveBeenCalledOnce();
});
it("leaves native typing undo and modal dialogs alone", () => {
  mount();
  for (const tag of ["input", "textarea"]) {
    const node = document.createElement(tag); document.body.append(node);
    expect(fireEvent.keyDown(node, { key: "z", ctrlKey: true })).toBe(true);
  }
  const editable = document.createElement("div"); editable.setAttribute("contenteditable", "true"); document.body.append(editable);
  fireEvent.keyDown(editable, { key: "z", ctrlKey: true });
  const dialog = document.createElement("dialog"); dialog.setAttribute("open", ""); document.body.append(dialog);
  fireEvent.keyDown(window, { key: "z", ctrlKey: true });
  expect(mock.call).not.toHaveBeenCalled();
});
it("waits for a queued edit and refuses duplicate undo while pending", async () => {
  let release!: () => void;
  const write = queueFieldWrite(1, () => new Promise<void>(resolve => { release = resolve; }));
  const { result } = mount();
  fireEvent.keyDown(window, { key: "z", ctrlKey: true });
  fireEvent.keyDown(window, { key: "z", ctrlKey: true });
  await act(async () => { await Promise.resolve(); });
  expect(mock.call).not.toHaveBeenCalled();
  await act(async () => { release(); await write; });
  await waitFor(() => expect(result.current.pending).toBe(false));
  expect(mock.call.mock.calls.filter(([command]) => command === "undo")).toHaveLength(1);
});
it("reports failures and allows a retry", async () => {
  mock.call.mockRejectedValueOnce(new Error("Undo refused"));
  const { result } = mount();
  await act(async () => { await result.current.run("undo"); });
  expect(result.current.error).toContain("Undo refused");
  await act(async () => { await result.current.run("undo"); });
  expect(result.current.error).toBeNull();
});
it("does not undo during save or close", async () => {
  const { result } = mount(true);
  await act(async () => { await result.current.run("undo"); });
  fireEvent.keyDown(window, { key: "z", ctrlKey: true });
  expect(mock.call).not.toHaveBeenCalled();
});
