import { expect, it, vi } from "vitest";
const ipc = vi.hoisted(() => vi.fn(async (..._args: unknown[]) => ({})));
vi.mock("@tauri-apps/api/core", () => ({ invoke: ipc }));
import { call } from "./client";
import { queueFieldWrite, awaitFieldWrites } from "./fieldWrites";

it.each(["project_save", "project_build", "undo", "redo", "close_project", "project_archive"] as const)("%s waits for queued field edits", async command => {
  ipc.mockClear();
  let finish!: () => void;
  const write = queueFieldWrite(701, () => new Promise<void>(resolve => { finish = resolve; }));
  await Promise.resolve();
  const action = call(command, { projectId: 701, projectsRoot: "test" } as never);
  await Promise.resolve();
  expect(ipc).not.toHaveBeenCalled();
  finish(); await write; await action;
  expect(ipc).toHaveBeenCalledTimes(1);
  expect(ipc.mock.calls[0][0]).toBe(command);
});

it("a refused field does not discard later edits or block saving", async () => {
  const order: string[] = [];
  const rejected = queueFieldWrite(702, async () => { order.push("refused"); throw new Error("Invalid"); }).catch(() => {});
  const accepted = queueFieldWrite(702, async () => { order.push("accepted"); });
  await awaitFieldWrites(702);
  await Promise.all([rejected, accepted]);
  expect(order).toEqual(["refused", "accepted"]);
});
