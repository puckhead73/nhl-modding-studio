// @vitest-environment jsdom
//
// LEAGUE-SMOKE-R1 — acknowledged project close. Both close paths (the shell's Close button and
// the picker's corruption-escape notice) dispose of the shell's owner only once the server has
// answered `closed` / `already_closed`; `busy`, `failed`, a rejected IPC call, a repeated click
// and a late response after a project switch must never drop the wrong session or hide a live
// operation. Component evidence only (jsdom, deferred mocked responses): not a desktop run.
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter, Route, Routes, useLocation } from "react-router-dom";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { CloseProjectDto } from "../bindings/CloseProjectDto";
import type { ProjectSummaryDto } from "../bindings/ProjectSummaryDto";

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

const calls: Array<{ cmd: string; args: { projectId: number } }> = [];
let respond: (args: { projectId: number }) => Promise<CloseProjectDto>;
vi.mock("../lib/client", () => ({
  call: (cmd: string, args: { projectId: number }) => {
    calls.push({ cmd, args });
    if (cmd !== "close_project") return Promise.reject(new Error(`unmocked command ${cmd}`));
    return respond(args);
  },
}));

let projectState: { project: ProjectSummaryDto | null; close: () => void };
vi.mock("../lib/project", () => ({
  useProject: () => projectState,
}));

import { CloseProjectButton } from "./CloseProjectButton";
import { LingeringProjectNotice } from "./LingeringProjectNotice";

function summary(project_id: number): ProjectSummaryDto {
  return { project_id, game_id: "nhl21", platform: "ps4" } as unknown as ProjectSummaryDto;
}
const outcome = (projectId: number, outcome: string, reason: string | null = null): CloseProjectDto => ({
  project_id: projectId,
  outcome,
  reason,
});

/** Records the current route so the tests can assert navigation (or its absence). */
const seen = { path: "" };
function RouteSpy() {
  seen.path = useLocation().pathname;
  return null;
}

function mountButton(projectId: number, start = "/app/database/pending") {
  return render(
    <MemoryRouter initialEntries={[start]}>
      <RouteSpy />
      <Routes>
        <Route path="/" element={<div>picker</div>} />
        <Route path="/app/database/pending" element={<CloseProjectButton projectId={projectId} />} />
      </Routes>
    </MemoryRouter>,
  );
}

function mountNotice() {
  return render(
    <MemoryRouter initialEntries={["/"]}>
      <RouteSpy />
      <Routes>
        <Route path="/" element={<LingeringProjectNotice />} />
        <Route path="/app/database/pending" element={<div>shell</div>} />
      </Routes>
    </MemoryRouter>,
  );
}

const closeCalls = () => calls.filter((c) => c.cmd === "close_project");

beforeEach(() => {
  calls.length = 0;
  seen.path = "";
  projectState = { project: summary(1), close: vi.fn() };
});
afterEach(() => cleanup());

describe("CloseProjectButton (shell Close)", () => {
  it("leaves the project only after the server acknowledged `closed`", async () => {
    const d = deferred<CloseProjectDto>();
    respond = () => d.promise;
    mountButton(1);
    fireEvent.click(screen.getByText("Close"));
    await waitFor(() => expect(screen.getByText("Closing…")).toBeTruthy());
    // Nothing has been disposed of while the request is in flight.
    expect(projectState.close).not.toHaveBeenCalled();
    expect(seen.path).toBe("/app/database/pending");
    await act(async () => {
      d.resolve(outcome(1, "closed"));
      await d.promise;
    });
    await waitFor(() => expect(projectState.close).toHaveBeenCalledTimes(1));
    expect(seen.path).toBe("/");
  });

  it("treats `already_closed` as a completed close (idempotent)", async () => {
    respond = ({ projectId }) => Promise.resolve(outcome(projectId, "already_closed"));
    mountButton(1);
    fireEvent.click(screen.getByText("Close"));
    await waitFor(() => expect(projectState.close).toHaveBeenCalledTimes(1));
    expect(seen.path).toBe("/");
  });

  it("keeps the owner and the route on `busy`, shows the reason, and offers a retry", async () => {
    let n = 0;
    respond = ({ projectId }) =>
      Promise.resolve(
        ++n === 1
          ? outcome(projectId, "busy", "apply of review handle 3 (project 1) still needs this session; wait for it to finish or cancel it, then close again")
          : outcome(projectId, "closed"),
      );
    mountButton(1);
    fireEvent.click(screen.getByText("Close"));
    await waitFor(() => expect(screen.getByRole("alert").textContent).toMatch(/still in use by a running operation.*apply of review handle 3/));
    expect(projectState.close).not.toHaveBeenCalled();
    expect(seen.path).toBe("/app/database/pending");
    // The button is usable again as a retry; the second attempt succeeds.
    fireEvent.click(screen.getByText("Retry close"));
    await waitFor(() => expect(projectState.close).toHaveBeenCalledTimes(1));
    expect(closeCalls()).toHaveLength(2);
    expect(seen.path).toBe("/");
  });

  it("keeps the owner on `failed` with the discard reason", async () => {
    respond = ({ projectId }) => Promise.resolve(outcome(projectId, "failed", "a prepared copied-target review could not be discarded (engine busy)"));
    mountButton(1);
    fireEvent.click(screen.getByText("Close"));
    await waitFor(() => expect(screen.getByRole("alert").textContent).toMatch(/could not be closed.*engine busy/));
    expect(projectState.close).not.toHaveBeenCalled();
    expect(seen.path).toBe("/app/database/pending");
  });

  it("keeps the owner when the IPC call itself rejects", async () => {
    respond = () => Promise.reject(new Error("ipc down"));
    mountButton(1);
    fireEvent.click(screen.getByText("Close"));
    await waitFor(() => expect(screen.getByRole("alert").textContent).toMatch(/Close did not complete: Error: ipc down/));
    expect(projectState.close).not.toHaveBeenCalled();
    expect(seen.path).toBe("/app/database/pending");
    expect(screen.getByText("Retry close")).toBeTruthy();
  });

  it("sends one request for repeated clicks while a close is pending", async () => {
    const d = deferred<CloseProjectDto>();
    respond = () => d.promise;
    mountButton(1);
    const button = screen.getByText("Close");
    fireEvent.click(button);
    fireEvent.click(button);
    fireEvent.click(button);
    await waitFor(() => expect(screen.getByText("Closing…")).toBeTruthy());
    fireEvent.click(screen.getByText("Closing…"));
    expect(closeCalls()).toHaveLength(1);
    await act(async () => {
      d.resolve(outcome(1, "closed"));
      await d.promise;
    });
    await waitFor(() => expect(projectState.close).toHaveBeenCalledTimes(1));
    expect(closeCalls()).toHaveLength(1);
  });

  it("ignores a late response after the project switched and never closes the new project", async () => {
    const d = deferred<CloseProjectDto>();
    respond = () => d.promise;
    const r = mountButton(1);
    fireEvent.click(screen.getByText("Close"));
    await waitFor(() => expect(closeCalls()).toHaveLength(1));
    // The shell now shows project 2 (the old one is no longer this button's owner).
    projectState = { project: summary(2), close: vi.fn() };
    r.rerender(
      <MemoryRouter initialEntries={["/app/database/pending"]}>
        <RouteSpy />
        <Routes>
          <Route path="/" element={<div>picker</div>} />
          <Route path="/app/database/pending" element={<CloseProjectButton projectId={2} />} />
        </Routes>
      </MemoryRouter>,
    );
    await act(async () => {
      d.resolve(outcome(1, "closed"));
      await d.promise;
    });
    // Neither the old nor the new owner is disposed of by the stale answer; the shell stays.
    expect(projectState.close).not.toHaveBeenCalled();
    expect(seen.path).toBe("/app/database/pending");
    // The new project's button is idle and usable.
    expect(screen.getByText("Close")).toBeTruthy();
  });
});

describe("LingeringProjectNotice (corruption escape on the picker)", () => {
  it("asks once on mount and clears the owner only on acknowledgement", async () => {
    const d = deferred<CloseProjectDto>();
    respond = () => d.promise;
    mountNotice();
    await waitFor(() => expect(closeCalls()).toEqual([{ cmd: "close_project", args: { projectId: 1 } }]));
    expect(screen.getByRole("status").textContent).toMatch(/Closing the previous project/);
    expect(projectState.close).not.toHaveBeenCalled();
    await act(async () => {
      d.resolve(outcome(1, "closed"));
      await d.promise;
    });
    await waitFor(() => expect(projectState.close).toHaveBeenCalledTimes(1));
    // No redirect: the picker route stays put.
    expect(seen.path).toBe("/");
  });

  it("on refusal keeps the owner, stays on the picker, and offers retry and a way back to the live project", async () => {
    let n = 0;
    respond = ({ projectId }) =>
      Promise.resolve(++n === 1 ? outcome(projectId, "busy", "restore of transaction 000004 still needs this session") : outcome(projectId, "closed"));
    mountNotice();
    await waitFor(() => expect(screen.getByRole("alert").textContent).toMatch(/still open.*restore of transaction 000004/));
    expect(projectState.close).not.toHaveBeenCalled();
    expect(seen.path).toBe("/");
    // Return to the project is a plain navigation into the still-open shell (no re-open, no close).
    fireEvent.click(screen.getByText("Return to project"));
    await waitFor(() => expect(seen.path).toBe("/app/database/pending"));
    expect(projectState.close).not.toHaveBeenCalled();
    expect(closeCalls()).toHaveLength(1);
  });

  it("retry after a refusal completes the close without a second automatic request", async () => {
    let n = 0;
    respond = ({ projectId }) => Promise.resolve(++n === 1 ? outcome(projectId, "busy", "held") : outcome(projectId, "closed"));
    mountNotice();
    await waitFor(() => expect(screen.getByText("Retry close")).toBeTruthy());
    expect(closeCalls()).toHaveLength(1);
    fireEvent.click(screen.getByText("Retry close"));
    await waitFor(() => expect(projectState.close).toHaveBeenCalledTimes(1));
    expect(closeCalls()).toHaveLength(2);
    expect(seen.path).toBe("/");
  });

  it("a rejected IPC call leaves the owner in place with a retry", async () => {
    respond = () => Promise.reject(new Error("ipc down"));
    mountNotice();
    await waitFor(() => expect(screen.getByRole("alert").textContent).toMatch(/Close did not complete/));
    expect(projectState.close).not.toHaveBeenCalled();
    expect(screen.getByText("Retry close")).toBeTruthy();
  });

  it("renders nothing when no project lingers", () => {
    projectState = { project: null, close: vi.fn() };
    respond = () => Promise.reject(new Error("must not be called"));
    const r = mountNotice();
    expect(r.container.textContent).toBe("");
    expect(closeCalls()).toHaveLength(0);
  });
});
