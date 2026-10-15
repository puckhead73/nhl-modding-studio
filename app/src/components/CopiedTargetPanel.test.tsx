// @vitest-environment jsdom
//
// LEAGUE-R1 F3 — review ownership and lifetime, driven with deferred API responses. Component
// evidence only (jsdom): not a desktop or in-game run.
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ProjectSummaryDto } from "../bindings/ProjectSummaryDto";
import type { CopiedTargetReviewDto } from "../bindings/CopiedTargetReviewDto";
import type { CopiedTargetStatusDto } from "../bindings/CopiedTargetStatusDto";

// ---- mocks ------------------------------------------------------------------------------------
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

const calls: Array<{ cmd: string; args: unknown }> = [];
const handlers: Record<string, (args: any) => unknown> = {};
vi.mock("../lib/client", () => ({
  call: (cmd: string, args: unknown) => {
    calls.push({ cmd, args });
    const h = handlers[cmd];
    if (!h) return Promise.reject(new Error(`unmocked command ${cmd}`));
    return Promise.resolve().then(() => h(args));
  },
}));

let projectState: { project: ProjectSummaryDto; update: (s: ProjectSummaryDto) => void };
vi.mock("../lib/project", () => ({
  useProject: () => projectState,
}));

import { CopiedTargetPanel } from "./CopiedTargetPanel";

function summary(over: Partial<ProjectSummaryDto>): ProjectSummaryDto {
  return {
    project_id: 1,
    game_id: "nhl21",
    platform: "ps4",
    role: "nhlng",
    source: "raw_db",
    endian: "Little",
    table_count: 1,
    db_size: 1,
    header_crc_ok: true,
    size_ok: true,
    dirty: true,
    undo_depth: 1,
    redo_depth: 0,
    tables: [],
    roster_attached: false,
    roster_aligned: false,
    display_roster: false,
    capabilities: {} as ProjectSummaryDto["capabilities"],
    surfaces: {} as ProjectSummaryDto["surfaces"],
    write_experimental: false,
    workspace_revision: "rev-a",
    ...over,
  };
}

function status(over: Partial<CopiedTargetStatusDto> = {}): CopiedTargetStatusDto {
  return {
    root: "C:/copy",
    transactions: [],
    blocked: false,
    active_txn_id: null,
    pending_handles: [],
    busy_projects: [],
    reservations: [],
    staging_leftovers: [],
    apply_allowed: true,
    guidance: "the root accepts a new copied-target review",
    ...over,
  };
}

function review(handle: number, projectId = 1): CopiedTargetReviewDto {
  return {
    handle,
    project_id: projectId,
    txn_id: `txn-${handle}`,
    root: "C:/copy",
    workspace_revision: "rev-a",
    mode: "copied-target",
    qualification_state: "unqualified",
    live_install_allowed: false,
    artifacts: [],
    bound_inputs: [],
    status: status({ pending_handles: [{ handle, project_id: projectId, txn_id: `txn-${handle}`, in_progress: false }] }),
  };
}

function mount() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <CopiedTargetPanel />
    </QueryClientProvider>,
  );
}

const discards = () => calls.filter((c) => c.cmd === "copied_target_discard").map((c) => (c.args as { handle: number }).handle);
const applies = () => calls.filter((c) => c.cmd === "copied_target_apply");

beforeEach(() => {
  calls.length = 0;
  for (const k of Object.keys(handlers)) delete handlers[k];
  handlers.copied_target_root_for = ({ projectId }) => ({ project_id: projectId, root: "C:/copy", note: null });
  handlers.copied_target_status = () => status();
  handlers.copied_target_discard = () => status();
  handlers.project_summary = ({ projectId }) => summary({ project_id: projectId });
  projectState = { project: summary({}), update: vi.fn() };
});
afterEach(() => cleanup());

describe("CopiedTargetPanel review ownership", () => {
  it("disposes of a review that arrives after the panel unmounted", async () => {
    const pending = deferred<CopiedTargetReviewDto>();
    handlers.copied_target_review = () => pending.promise;
    const r = mount();
    await waitFor(() => expect((screen.getByLabelText(/Copied-target root/) as HTMLInputElement).value).toBe("C:/copy"));
    fireEvent.click(screen.getByText("Review save"));
    await waitFor(() => expect(calls.some((c) => c.cmd === "copied_target_review")).toBe(true));
    r.unmount();
    await act(async () => {
      pending.resolve(review(7));
      await pending.promise;
    });
    await waitFor(() => expect(discards()).toEqual([7]));
    expect(applies()).toHaveLength(0);
  });

  it("disposes of a review that arrives after a project switch and never updates the new project", async () => {
    const pending = deferred<CopiedTargetReviewDto>();
    handlers.copied_target_review = () => pending.promise;
    const r = mount();
    await waitFor(() => expect((screen.getByLabelText(/Copied-target root/) as HTMLInputElement).value).toBe("C:/copy"));
    fireEvent.click(screen.getByText("Review save"));
    await waitFor(() => expect(calls.some((c) => c.cmd === "copied_target_review")).toBe(true));
    // Switch projects while the review is in flight.
    const update = vi.fn();
    projectState = { project: summary({ project_id: 2, workspace_revision: "rev-p2" }), update };
    r.rerender(
      <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
        <CopiedTargetPanel />
      </QueryClientProvider>,
    );
    await act(async () => {
      pending.resolve(review(9, 1));
      await pending.promise;
    });
    await waitFor(() => expect(discards()).toContain(9));
    expect(screen.queryByLabelText("prepared review")).toBeNull();
    expect(update).not.toHaveBeenCalled();
  });

  it("freezes the destination controls while a review is prepared and unfreezes on discard", async () => {
    handlers.copied_target_review = () => review(3);
    mount();
    await waitFor(() => expect((screen.getByLabelText(/Copied-target root/) as HTMLInputElement).value).toBe("C:/copy"));
    const root = screen.getByLabelText(/Copied-target root/) as HTMLInputElement;
    const archive = screen.getByLabelText(/Also replace the database member/) as HTMLInputElement;
    expect(root.disabled).toBe(false);
    fireEvent.click(screen.getByText("Review save"));
    await waitFor(() => expect(screen.getByLabelText("prepared review")).toBeTruthy());
    expect(root.disabled).toBe(true);
    expect(archive.disabled).toBe(true);
    expect((screen.getByText("Apply to copied target") as HTMLButtonElement).disabled).toBe(false);
    fireEvent.click(screen.getByText("Discard review"));
    await waitFor(() => expect(discards()).toEqual([3]));
    await waitFor(() => expect(screen.queryByLabelText("prepared review")).toBeNull());
    expect(root.disabled).toBe(false);
    expect(archive.disabled).toBe(false);
    // Enabling the archive now mints a review through the members command (one `primary` spec),
    // not the loose one.
    handlers.copied_target_review_archive_members = () => review(4);
    fireEvent.click(archive);
    fireEvent.click(screen.getByText("Review save"));
    await waitFor(() => expect(calls.some((c) => c.cmd === "copied_target_review_archive_members")).toBe(true));
    const members = calls.find((c) => c.cmd === "copied_target_review_archive_members")! as { cmd: string; args: { specs: unknown } };
    expect(members.args.specs).toEqual([
      { archive_relative: "databigs/inxp_00.big", member_path: "nhl/data/common/gamemodes/database/t3db/nhl_game.db", artifact_key: "primary" },
    ]);
    expect(applies()).toHaveLength(0);
  });

  it("adds the dictionary member spec (LOC-W1) beside the database member", async () => {
    handlers.copied_target_review_archive_members = () => review(12);
    mount();
    await waitFor(() => expect((screen.getByLabelText(/Copied-target root/) as HTMLInputElement).value).toBe("C:/copy"));
    fireEvent.click(screen.getByLabelText(/Also replace the database member/));
    fireEvent.click(screen.getByLabelText(/Also replace the dictionary member/));
    fireEvent.click(screen.getByText("Review save"));
    await waitFor(() => expect(calls.some((c) => c.cmd === "copied_target_review_archive_members")).toBe(true));
    const members = calls.find((c) => c.cmd === "copied_target_review_archive_members")! as { cmd: string; args: { specs: unknown } };
    expect(members.args.specs).toEqual([
      { archive_relative: "databigs/inxp_00.big", member_path: "nhl/data/common/gamemodes/database/t3db/nhl_game.db", artifact_key: "primary" },
      { archive_relative: "databigs/inxp_00.big", member_path: "nhl/data/kettle/loc/eng_us.db", artifact_key: "dictionary" },
    ]);
  });

  it("marks a review stale on a revision change even when undo/redo depths match", async () => {
    handlers.copied_target_review = () => review(5);
    const r = mount();
    await waitFor(() => expect((screen.getByLabelText(/Copied-target root/) as HTMLInputElement).value).toBe("C:/copy"));
    fireEvent.click(screen.getByText("Review save"));
    await waitFor(() => expect(screen.getByLabelText("prepared review")).toBeTruthy());
    expect((screen.getByText("Apply to copied target") as HTMLButtonElement).disabled).toBe(false);
    // edit → undo → a different edit: same depths and dirty flag, different content revision.
    projectState = { ...projectState, project: summary({ undo_depth: 1, redo_depth: 0, dirty: true, workspace_revision: "rev-b" }) };
    r.rerender(
      <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
        <CopiedTargetPanel />
      </QueryClientProvider>,
    );
    await waitFor(() => expect(screen.getByRole("status").textContent).toMatch(/stale/));
    expect((screen.getByText("Apply to copied target") as HTMLButtonElement).disabled).toBe(true);
    fireEvent.click(screen.getByText("Apply to copied target"));
    expect(applies()).toHaveLength(0);
  });

  it("keeps cancel available while an apply is in flight and refreshes only the owning project", async () => {
    handlers.copied_target_review = () => review(6);
    const applying = deferred<unknown>();
    handlers.copied_target_apply = () => applying.promise;
    handlers.copied_target_cancel = () => true;
    const update = vi.fn();
    projectState = { ...projectState, update };
    mount();
    await waitFor(() => expect((screen.getByLabelText(/Copied-target root/) as HTMLInputElement).value).toBe("C:/copy"));
    fireEvent.click(screen.getByText("Review save"));
    await waitFor(() => expect(screen.getByLabelText("prepared review")).toBeTruthy());
    fireEvent.click(screen.getByText("Apply to copied target"));
    await waitFor(() => expect(screen.getByText("Cancel apply")).toBeTruthy());
    fireEvent.click(screen.getByText("Cancel apply"));
    await waitFor(() => expect(calls.some((c) => c.cmd === "copied_target_cancel")).toBe(true));
    await act(async () => {
      applying.resolve({
        handle: 6,
        txn_id: "txn-6",
        state: "committed",
        recovery_dir: "C:/copy/.tdb-txn/txn-6",
        destinations: [],
        applied_unqualified: true,
        project: { project_id: 1, primary: "rebound", roster: null, dirty: false, stale_baseline: null, refresh_error: null },
        aliases: [],
        log: [],
        status: status(),
      });
      await applying.promise;
    });
    await waitFor(() => expect(update).toHaveBeenCalled());
    expect((update.mock.calls[0][0] as ProjectSummaryDto).project_id).toBe(1);
  });

  // LEAGUE-RESP-1 — once cancellation settles in under a second, the panel's old order (clear the
  // owner, then refresh everything) showed the cached pre-apply status — its own, now-aborted
  // handle as a foreign "review pending" with a Discard button — until every query had refetched.
  // The status is re-read before ownership is dropped; the abort message is not delayed by the
  // other refetches.
  it("re-reads the root status before dropping a cancelled apply's handle — never the pre-apply cache", async () => {
    handlers.copied_target_review = () => review(9);
    let statusReads = 0;
    // Status while prepared: the handle is listed; after the apply: the root is free again.
    let applied = false;
    handlers.copied_target_status = () => {
      statusReads += 1;
      return applied ? status() : status({ active_txn_id: "txn-9", apply_allowed: false, pending_handles: [{ handle: 9, project_id: 1, txn_id: "txn-9", in_progress: false }] });
    };
    const applying = deferred<unknown>();
    handlers.copied_target_apply = () => applying.promise;
    handlers.copied_target_cancel = () => true;
    // The summary refetch is slow (it queues behind the session lock in the real app).
    const slowSummary = deferred<ProjectSummaryDto>();
    handlers.project_summary = ({ projectId }) => (applied ? slowSummary.promise : summary({ project_id: projectId }));
    mount();
    await waitFor(() => expect((screen.getByLabelText(/Copied-target root/) as HTMLInputElement).value).toBe("C:/copy"));
    fireEvent.click(screen.getByText("Review save"));
    await waitFor(() => expect(screen.getByLabelText("prepared review")).toBeTruthy());
    fireEvent.click(screen.getByText("Apply to copied target"));
    await waitFor(() => expect(screen.getByText("Cancel apply")).toBeTruthy());
    fireEvent.click(screen.getByText("Cancel apply"));
    await waitFor(() => expect(screen.getByText("Cancellation requested.")).toBeTruthy());
    const readsBefore = statusReads;
    applied = true;
    await act(async () => {
      applying.reject(new Error("transaction txn-9: cancelled at BeforeStageReplacement; Aborted; nothing was changed"));
      await applying.promise.catch(() => {});
    });
    // The abort message shows without waiting for the slow summary, and the status was re-read.
    await waitFor(() => expect(screen.getByText(/cancelled at BeforeStageReplacement/)).toBeTruthy());
    expect(statusReads).toBeGreaterThan(readsBefore);
    expect(screen.queryByText("Cancel apply")).toBeNull();
    expect(screen.queryByLabelText("other prepared reviews")).toBeNull();
    expect(screen.queryByText(/review pending/)).toBeNull();
    expect(screen.getByText(/the root accepts a new copied-target review/)).toBeTruthy();
    expect(discards()).toEqual([]);
    slowSummary.resolve(summary({ project_id: 1 }));
  });

  it("surfaces a failed disposal and lists foreign prepared handles from the root status", async () => {
    handlers.copied_target_review = () => review(8);
    handlers.copied_target_discard = () => {
      throw new Error("engine busy");
    };
    handlers.copied_target_status = () =>
      status({ pending_handles: [{ handle: 42, project_id: 3, txn_id: "txn-42", in_progress: false }] });
    mount();
    await waitFor(() => expect(screen.getByLabelText("other prepared reviews")).toBeTruthy());
    fireEvent.click(screen.getByText("Review save"));
    await waitFor(() => expect(screen.getByLabelText("prepared review")).toBeTruthy());
    fireEvent.click(screen.getByText("Discard review"));
    await waitFor(() => expect(screen.getByLabelText("cleanup failures").textContent).toMatch(/handle 8 .*could not be discarded/));
  });
});

// LEAGUE-SMOKE-1 — status presentation. The engine reports a prepared transaction as blocking
// the root (no second review may start); the panel must show its own review as "review prepared"
// and must not offer recovery for a transaction a live handle owns. Recovery stays offered for a
// genuinely interrupted transaction (an active id with no handle behind it).
describe("CopiedTargetPanel status presentation", () => {
  it("shows its own prepared review as prepared, not as a blocked root needing recovery", async () => {
    handlers.copied_target_review = () => review(11);
    // After the review the root's status names the prepared transaction as the active one.
    handlers.copied_target_status = () =>
      calls.some((c) => c.cmd === "copied_target_review")
        ? status({
            blocked: true,
            apply_allowed: false,
            active_txn_id: "txn-11",
            pending_handles: [{ handle: 11, project_id: 1, txn_id: "txn-11", in_progress: false }],
            guidance: "review handle 11 is prepared for transaction txn-11; apply it or discard it before starting another",
          })
        : status();
    mount();
    await waitFor(() => expect(screen.getByText("ready")).toBeTruthy());
    fireEvent.click(screen.getByText("Review save"));
    await waitFor(() => expect(screen.getByLabelText("prepared review")).toBeTruthy());
    await waitFor(() => expect(screen.getByText("review prepared")).toBeTruthy());
    expect(screen.queryByText("blocked")).toBeNull();
    expect(screen.queryByText("Run recovery")).toBeNull();
    expect((screen.getByText("Apply to copied target") as HTMLButtonElement).disabled).toBe(false);
  });

  it("shows another panel's review as pending and an in-flight apply as applying, without recovery", async () => {
    handlers.copied_target_status = () =>
      status({
        blocked: true,
        apply_allowed: false,
        active_txn_id: "txn-21",
        pending_handles: [{ handle: 21, project_id: 4, txn_id: "txn-21", in_progress: false }],
      });
    const r = mount();
    await waitFor(() => expect(screen.getByText("review pending")).toBeTruthy());
    expect(screen.queryByText("Run recovery")).toBeNull();
    expect(screen.getByLabelText("other prepared reviews").textContent).toMatch(/handle 21 for project 4/);
    r.unmount();
    handlers.copied_target_status = () =>
      status({
        blocked: false,
        apply_allowed: false,
        active_txn_id: "txn-22",
        pending_handles: [{ handle: 22, project_id: 4, txn_id: "txn-22", in_progress: true }],
        guidance: "an apply is running on this root; wait for it or cancel it",
      });
    mount();
    await waitFor(() => expect(screen.getByText("applying")).toBeTruthy());
    expect(screen.queryByText("Run recovery")).toBeNull();
  });

  it("still offers recovery for an interrupted transaction nobody holds", async () => {
    handlers.copied_target_status = () =>
      status({
        blocked: true,
        apply_allowed: false,
        active_txn_id: "txn-99",
        pending_handles: [],
        guidance: "transaction txn-99 is staged; run recovery inspection on this root before another copied-target save",
      });
    mount();
    await waitFor(() => expect(screen.getByText("blocked")).toBeTruthy());
    expect(screen.getByText("Run recovery")).toBeTruthy();
  });

  it("drops the earlier 'Created …' note when a later root creation is refused", async () => {
    let creations = 0;
    handlers.copied_target_create_root = ({ path }) => {
      creations += 1;
      if (creations === 1) return path;
      throw new Error(`${path} already exists; a copied-target root is only ever created as a new, empty directory`);
    };
    mount();
    await waitFor(() => expect((screen.getByLabelText(/Copied-target root/) as HTMLInputElement).value).toBe("C:/copy"));
    fireEvent.change(screen.getByLabelText("new copied root path"), { target: { value: "C:/new-root" } });
    fireEvent.click(screen.getByText("Create"));
    await waitFor(() => expect(screen.getByText(/Created the copied-target root C:\/new-root/)).toBeTruthy());
    fireEvent.click(screen.getByText("Create"));
    await waitFor(() => expect(screen.getByText(/already exists/)).toBeTruthy());
    expect(screen.queryByText(/Created the copied-target root/)).toBeNull();
  });
});
