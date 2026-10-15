// @vitest-environment jsdom
//
// p1.1-xbox360-iso-member-route — the Disc image panel: hidden without an image binding, refuses
// to write while the project is dirty, requires an explicit confirm, shows the apply outcome and
// the records, restores per record, and drops a late apply outcome after the project changed.
// R2 F2: every command names the project; the write names the reviewed plan; a confirm opened on
// one review is withdrawn when the project moves (dirty, revision, newer review); a busy image
// disables writes; failed applies show every attempted range and its state.
// Component evidence only (jsdom, mocked commands): not a desktop run.
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ProjectSummaryDto } from "../bindings/ProjectSummaryDto";
import type { IsoStagedDto } from "../bindings/IsoStagedDto";
import type { IsoPlanDto } from "../bindings/IsoPlanDto";
import type { IsoApplyDto } from "../bindings/IsoApplyDto";
import type { IsoRecordDto } from "../bindings/IsoRecordDto";

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

let projectState: { project: ProjectSummaryDto | null; iso: IsoStagedDto | null };
vi.mock("../lib/project", () => ({
  useProject: () => projectState,
}));

import { DiscImagePanel } from "./DiscImagePanel";

function summary(over: Partial<ProjectSummaryDto>): ProjectSummaryDto {
  return {
    project_id: 1,
    game_id: "nhl14",
    platform: "xbox",
    role: "nhlng",
    source: "raw_db",
    endian: "Big",
    table_count: 1,
    db_size: 1,
    header_crc_ok: true,
    size_ok: true,
    dirty: false,
    undo_depth: 0,
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

const staged: IsoStagedDto = {
  root: "E:/staging/nhl14",
  image: "X:/NHL 14.iso",
  layout: "xgd2",
  game_id: "nhl14",
  platform: "xbox",
  db_path: "E:/staging/nhl14/db/nhlng.db",
  meta_xml: "E:/staging/nhl14/db/nhlng-meta.xml",
  dictionary_path: "E:/staging/nhl14/fe/loc/nhl_eng_us.db",
  files: [],
  reused: false,
  staged_at: "2026-09-18T05:00:00Z",
  project_id: 1,
};

function plan(changed: boolean, over: Partial<IsoPlanDto> = {}): IsoPlanDto {
  return {
    review_id: BigInt(7),
    project_id: 1,
    workspace_revision: "rev-a",
    dirty: false,
    image: staged.image,
    root: staged.root,
    writes: changed ? 1 : 0,
    ...over,
    files: [
      {
        role: "dictionary",
        staged: "fe/loc/nhl_eng_us.db",
        staged_sha256: changed ? "bbbb" : "aaaa",
        changed,
        member_path: changed ? "cacheboot.big" : null,
        entry_path: changed ? "fe/loc/nhl_eng_us.db" : null,
        start: changed ? BigInt(100) : null,
        end: changed ? BigInt(200) : null,
        prior_sha256: changed ? "aaaa" : null,
        replacement_sha256: changed ? "bbbb" : null,
      },
    ],
  };
}

const idle = { project_id: 1, root: staged.root, image: staged.image, in_flight: null, pending_reviews: 1, cancellable: false, note: "a range write completes and verifies or is put back from its record" };

function record(onImage: string): IsoRecordDto {
  return {
    records_dir: "E:/staging/nhl14/records/0001",
    record_dir: "E:/staging/nhl14/records/0001/dictionary",
    image: staged.image,
    member_path: "cacheboot.big",
    entry_path: "fe/loc/nhl_eng_us.db",
    start: BigInt(100),
    end: BigInt(200),
    prior_sha256: "aaaa",
    replacement_sha256: "bbbb",
    status: "applied",
    on_image: onImage,
  };
}

const applied: IsoApplyDto = {
  outcome: "committed",
  message: "1 range(s) written and verified",
  records_dir: "E:/staging/nhl14/records/0001",
  records: [record("replacement")],
  attempted: [],
  unchanged: [],
  failed: null,
  cause: null,
};

function mount() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  const el = () => (
    <QueryClientProvider client={qc}>
      <DiscImagePanel />
    </QueryClientProvider>
  );
  const r = render(el());
  return { ...r, rerender: () => r.rerender(el()) };
}

const writeButton = () => screen.queryByText("Write to disc image…") as HTMLButtonElement | null;

beforeEach(() => {
  calls.length = 0;
  for (const k of Object.keys(handlers)) delete handlers[k];
  handlers.iso_plan = () => plan(true);
  handlers.iso_records = () => [];
  handlers.iso_status = () => idle;
  projectState = { project: summary({}), iso: staged };
});
afterEach(() => cleanup());

describe("DiscImagePanel", () => {
  it("renders nothing without an image binding", () => {
    projectState = { project: summary({}), iso: null };
    mount();
    expect(screen.queryByText("Disc image")).toBeNull();
    expect(calls).toHaveLength(0);
  });

  it("shows the plan and refuses to write while the project is dirty", async () => {
    projectState = { project: summary({ dirty: true }), iso: staged };
    mount();
    await waitFor(() => expect(screen.getByTestId("iso-plan")).toBeTruthy());
    expect(screen.getByText(/unsaved edits/)).toBeTruthy();
    expect(writeButton()!.disabled).toBe(true);
  });

  it("disables writing when nothing changed", async () => {
    handlers.iso_plan = () => plan(false);
    mount();
    await waitFor(() => expect(screen.getByText(/Nothing to write/)).toBeTruthy());
    expect(writeButton()!.disabled).toBe(true);
  });

  it("writes only after an explicit confirm, shows the outcome, then restores the record", async () => {
    handlers.iso_apply = () => applied;
    handlers.iso_restore = () => ({ records_dir: applied.records_dir, items: [{ entry_path: "fe/loc/nhl_eng_us.db", outcome: "restored" }], all_prior: true });
    mount();
    await waitFor(() => expect(writeButton()!.disabled).toBe(false));
    fireEvent.click(writeButton()!);
    expect(calls.some((c) => c.cmd === "iso_apply")).toBe(false);
    expect(screen.getByText(/Continue\?/)).toBeTruthy();
    fireEvent.click(screen.getByText("Cancel"));
    expect(calls.some((c) => c.cmd === "iso_apply")).toBe(false);
    // Once applied, the records query (invalidated by the apply) lists the written range.
    handlers.iso_records = () => (calls.some((c) => c.cmd === "iso_apply") ? [record("replacement")] : []);
    fireEvent.click(writeButton()!);
    fireEvent.click(screen.getByText("Yes, write"));
    await waitFor(() => expect(screen.getByTestId("iso-applied")).toBeTruthy());
    expect(calls.filter((c) => c.cmd === "iso_apply")).toHaveLength(1);
    const applyCall = calls.find((c) => c.cmd === "iso_apply")!;
    expect(applyCall.args.projectId).toBe(1);
    expect(applyCall.args.reviewId).toBe(BigInt(7));
    expect(calls.find((c) => c.cmd === "iso_plan")!.args).toEqual({ projectId: 1 });
    // Restore acts on that records directory, for this project.
    await waitFor(() => expect(screen.getByText("Restore")).toBeTruthy());
    fireEvent.click(screen.getByText("Restore"));
    await waitFor(() => expect(screen.getByTestId("iso-restored")).toBeTruthy());
    expect(calls.find((c) => c.cmd === "iso_restore")!.args).toEqual({ projectId: 1, recordsDir: applied.records_dir });
    expect(screen.getByText(/original bytes at every recorded range/)).toBeTruthy();
  });

  it("withdraws an open confirm when the project becomes dirty or its revision moves", async () => {
    handlers.iso_apply = () => applied;
    const r = mount();
    await waitFor(() => expect(writeButton()!.disabled).toBe(false));
    fireEvent.click(writeButton()!);
    expect(screen.getByText("Yes, write")).toBeTruthy();
    // An edit lands after the confirm opened: the confirm is gone and the write is disabled.
    projectState = { project: summary({ dirty: true, workspace_revision: "rev-b" }), iso: staged };
    r.rerender();
    await waitFor(() => expect(screen.queryByText("Yes, write")).toBeNull());
    expect(writeButton()!.disabled).toBe(true);
    expect(calls.some((c) => c.cmd === "iso_apply")).toBe(false);
    // Saved: the revision stays moved and the server issues a newer review for it; only that
    // review can be confirmed, and the confirm names it.
    handlers.iso_plan = () => plan(true, { review_id: BigInt(8), workspace_revision: "rev-b" });
    projectState = { project: summary({ dirty: false, workspace_revision: "rev-b" }), iso: staged };
    r.rerender();
    await waitFor(() => expect(screen.getByTestId("iso-plan").getAttribute("data-review")).toBe("8"));
    await waitFor(() => expect(writeButton()!.disabled).toBe(false));
    fireEvent.click(writeButton()!);
    fireEvent.click(screen.getByText("Yes, write"));
    await waitFor(() => expect(calls.some((c) => c.cmd === "iso_apply")).toBe(true));
    expect(calls.find((c) => c.cmd === "iso_apply")!.args.reviewId).toBe(BigInt(8));
  });

  it("refuses to write a review that the server marked dirty or that belongs to another revision", async () => {
    handlers.iso_plan = () => plan(true, { dirty: true });
    const r = mount();
    await waitFor(() => expect(screen.getByTestId("iso-plan")).toBeTruthy());
    expect(writeButton()!.disabled).toBe(true);
    handlers.iso_plan = () => plan(true, { workspace_revision: "rev-old" });
    projectState = { project: summary({ workspace_revision: "rev-b" }), iso: staged };
    r.rerender();
    await waitFor(() => expect(screen.getByText(/changed since this review/)).toBeTruthy());
    expect(writeButton()!.disabled).toBe(true);
  });

  it("disables writing and restoring while another operation holds the image", async () => {
    handlers.iso_status = () => ({ ...idle, in_flight: "disc-image write of review 3 (project 9)" });
    handlers.iso_records = () => [record("replacement")];
    mount();
    await waitFor(() => expect(screen.getByText(/busy: disc-image write of review 3/)).toBeTruthy());
    expect(writeButton()!.disabled).toBe(true);
    await waitFor(() => expect(screen.getByText("Restore")).toBeTruthy());
    expect((screen.getByText("Restore") as HTMLButtonElement).disabled).toBe(true);
  });

  it("offers Restore for replacement and partial ranges only; a mixed range is marked as changed after the write", async () => {
    handlers.iso_records = () => [
      record("replacement"),
      { ...record("partial"), record_dir: "E:/staging/nhl14/records/0001/nhlng", entry_path: "db/nhlng.db", status: "restoring" },
      { ...record("mixed"), record_dir: "E:/staging/nhl14/records/0002/dictionary", records_dir: "E:/staging/nhl14/records/0002", status: "restored" },
      { ...record("unknown"), record_dir: "E:/staging/nhl14/records/0003/dictionary", records_dir: "E:/staging/nhl14/records/0003" },
    ];
    mount();
    await waitFor(() => expect(screen.getByText("mixed")).toBeTruthy());
    expect(screen.getAllByText("Restore")).toHaveLength(2);
    expect(screen.getByText(/changed after this write/)).toBeTruthy();
    expect(screen.getByText("unknown")).toBeTruthy();
  });

  it("shows a rolled-back apply with every attempted range and its state, and offers restore for a partial range", async () => {
    handlers.iso_apply = () => ({
      ...applied,
      outcome: "rolled-back",
      message: "writing fe/loc/nhl_eng_us.db to the image failed: injected. Every attempted range was put back from its backup and verified (fe/loc/nhl_eng_us.db, db/nhlng.db), so the image is as it was",
      records: [record("prior"), { ...record("prior"), entry_path: "db/nhlng.db", record_dir: "E:/staging/nhl14/records/0001/nhlng" }],
      attempted: [
        { role: "dictionary", entry_path: "fe/loc/nhl_eng_us.db", record: record("prior"), state: "prior", restore_error: null },
        { role: "nhlng", entry_path: "db/nhlng.db", record: { ...record("prior"), entry_path: "db/nhlng.db", record_dir: "E:/staging/nhl14/records/0001/nhlng" }, state: "prior", restore_error: null },
      ],
      failed: "fe/loc/nhl_eng_us.db",
      cause: "injected",
    });
    mount();
    await waitFor(() => expect(writeButton()!.disabled).toBe(false));
    fireEvent.click(writeButton()!);
    fireEvent.click(screen.getByText("Yes, write"));
    await waitFor(() => expect(screen.getByTestId("iso-apply-failed")).toBeTruthy());
    expect(screen.getAllByText(/the image is as it was/).length).toBeGreaterThan(0);
    expect(screen.getAllByText("prior")).toHaveLength(2);
    expect(screen.queryByTestId("iso-applied")).toBeNull();
    // A split outcome with an interrupted range is listed as such, and the records table offers
    // Restore for it.
    handlers.iso_records = () => [{ ...record("partial"), status: "writing" }];
    handlers.iso_apply = () => ({
      ...applied,
      outcome: "split",
      message: "split",
      records: [{ ...record("partial"), status: "writing" }],
      attempted: [{ role: "dictionary", entry_path: "fe/loc/nhl_eng_us.db", record: { ...record("partial"), status: "writing" }, state: "partial", restore_error: "device gone" }],
      failed: "fe/loc/nhl_eng_us.db",
      cause: "device gone",
    });
    await waitFor(() => expect(writeButton()!.disabled).toBe(false));
    fireEvent.click(writeButton()!);
    fireEvent.click(screen.getByText("Yes, write"));
    await waitFor(() => expect(screen.getByText(/the image is split/)).toBeTruthy());
    expect(screen.getByText(/device gone/)).toBeTruthy();
    await waitFor(() => expect(screen.getByText("Restore")).toBeTruthy());
  });

  it("drops a late apply outcome after the project changed", async () => {
    const pending = deferred<IsoApplyDto>();
    handlers.iso_apply = () => pending.promise;
    const r = mount();
    await waitFor(() => expect(writeButton()!.disabled).toBe(false));
    fireEvent.click(writeButton()!);
    fireEvent.click(screen.getByText("Yes, write"));
    await waitFor(() => expect(calls.some((c) => c.cmd === "iso_apply")).toBe(true));
    projectState = { project: summary({ project_id: 2 }), iso: { ...staged, root: "E:/staging/other" } };
    r.rerender();
    await act(async () => {
      pending.resolve(applied);
      await pending.promise;
    });
    expect(screen.queryByTestId("iso-applied")).toBeNull();
  });
});
