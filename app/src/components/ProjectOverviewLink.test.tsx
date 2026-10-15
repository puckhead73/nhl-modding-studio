// @vitest-environment jsdom
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { MemoryRouter } from "react-router-dom";
import { afterEach, expect, it, vi } from "vitest";

const state = vi.hoisted(() => ({ undo: 0, dirty: false, saves: 0, count: 0, fail: false }));
vi.mock("../lib/project", () => ({ useProject: () => ({
  project: { project_id: 1, undo_depth: state.undo, redo_depth: 0, dirty: state.dirty },
  info: { saves: state.saves },
}) }));
vi.mock("../lib/client", () => ({ call: () => state.fail
  ? Promise.reject(new Error("Review unavailable"))
  : Promise.resolve({ changes: Array.from({ length: state.count }, (_, id) => ({ id })) }),
}));
import { ProjectOverviewLink } from "./ProjectOverviewLink";
afterEach(() => { cleanup(); Object.assign(state, { undo: 0, dirty: false, saves: 0, count: 0, fail: false }); });

it("counts net edits, returns to zero on manual reversion, and retains saved edits", async () => {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  const view = () => <MemoryRouter><QueryClientProvider client={qc}><ProjectOverviewLink /></QueryClientProvider></MemoryRouter>;
  const { rerender } = render(view());
  const expectCount = (count: number) => waitFor(() => expect(screen.getByRole("link").textContent).toBe(`Project Overview${count}`));
  await expectCount(0);
  Object.assign(state, { undo: 1, dirty: true, count: 1 });
  rerender(view());
  await expectCount(1);
  // Typing the original name adds an undo operation but removes the net change.
  Object.assign(state, { undo: 2, dirty: false, count: 0 });
  rerender(view());
  await expectCount(0);
  Object.assign(state, { undo: 3, dirty: true, count: 1 });
  rerender(view());
  await expectCount(1);
  Object.assign(state, { dirty: false, saves: 1 });
  rerender(view());
  await expectCount(1);
  // A failed refresh must not present an old count as authoritative.
  state.fail = true;
  await qc.invalidateQueries({ queryKey: ["project-overview"] });
  await waitFor(() => expect(screen.getByRole("link").textContent).toBe("Project Overview"));
});
