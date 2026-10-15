// @vitest-environment jsdom
//
// LEAGUE-PARITY-COMPLETION-R1 F3 — the rename panel's async ownership, driven with deferred API
// responses. Component evidence only (jsdom): not a desktop or in-game run.
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ProjectSummaryDto } from "../bindings/ProjectSummaryDto";
import type { DictionaryRepointPreviewDto } from "../bindings/DictionaryRepointPreviewDto";

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

let projectState: { update: (s: ProjectSummaryDto) => void };
vi.mock("../lib/project", () => ({
  useProject: () => projectState,
}));

import { DictionaryRenamePanel } from "./DictionaryRenamePanel";

function summary(over: Partial<ProjectSummaryDto>): ProjectSummaryDto {
  return {
    project_id: 1,
    game_id: "nhllegacy",
    platform: "ps3",
    role: "nhlng",
    source: "raw_db",
    endian: "Big",
    table_count: 1,
    db_size: 1,
    header_crc_ok: true,
    size_ok: true,
    dirty: true,
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

function preview(key: string, donor: string, after = "Rest Of World"): DictionaryRepointPreviewDto {
  return {
    source: { record: 10, key, hash: 1, text: "Norway" },
    donor: { record: 20, key: donor, hash: 2, text: after },
    source_lookup: "exact",
    donor_lookup: "exact",
    text_before: "Norway",
    text_after: after,
    endian_big: true,
    dictionary_path: "C:/copy/USRDIR/fe/loc/nhl_eng_us.db",
    dictionary_sha256: "d1",
    slot_offset: 100,
    old_pointer: 1,
    new_pointer: 2,
    hash_collisions: [],
    sharing_old_text: [],
    sharing_new_text: [],
    warnings: [],
  } as unknown as DictionaryRepointPreviewDto;
}

function mount(props: { projectId?: number; dictionarySha256?: string } = {}) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  const el = (p: { projectId?: number; dictionarySha256?: string }) => (
    <QueryClientProvider client={qc}>
      <DictionaryRenamePanel
        projectId={p.projectId ?? 1}
        initialKey="TXT_LEAGUE_NORWAY"
        keyEvidence="hypothesis"
        dictionarySha256={p.dictionarySha256 ?? "d1"}
      />
    </QueryClientProvider>
  );
  const r = render(el(props));
  return { ...r, rerenderWith: (p: { projectId?: number; dictionarySha256?: string }) => r.rerender(el(p)) };
}

const keyInput = () => screen.getByLabelText("dictionary key to change") as HTMLInputElement;
const donorInput = () => screen.getByLabelText("donor dictionary key") as HTMLInputElement;
const shownPreview = () => screen.queryByTestId("rename-preview");

beforeEach(() => {
  calls.length = 0;
  for (const k of Object.keys(handlers)) delete handlers[k];
  handlers.project_summary = ({ projectId }) => summary({ project_id: projectId });
  projectState = { update: vi.fn() };
});
afterEach(() => cleanup());

describe("DictionaryRenamePanel ownership", () => {
  it("shows a preview for the current inputs and drops it when either input changes", async () => {
    handlers.dictionary_rename_preview = ({ key, donor }) => preview(key, donor);
    mount();
    fireEvent.click(screen.getByText("Preview"));
    await waitFor(() => expect(shownPreview()).toBeTruthy());
    expect(screen.getByText("Rest Of World")).toBeTruthy();
    fireEvent.change(donorInput(), { target: { value: "TXT_LEAGUE_SWEDEN" } });
    expect(shownPreview()).toBeNull();
    fireEvent.change(donorInput(), { target: { value: "TXT_ROW_LEAGUE" } });
    // Typing the old value back does not resurrect a dropped preview: a fresh request is needed.
    expect(shownPreview()).toBeNull();
  });

  it("ignores a preview that arrives after the inputs changed while it was pending", async () => {
    const pending = deferred<DictionaryRepointPreviewDto>();
    handlers.dictionary_rename_preview = () => pending.promise;
    mount();
    fireEvent.click(screen.getByText("Preview"));
    await waitFor(() => expect(calls.some((c) => c.cmd === "dictionary_rename_preview")).toBe(true));
    fireEvent.change(keyInput(), { target: { value: "TXT_LEAGUE_SWEDEN" } });
    await act(async () => {
      pending.resolve(preview("TXT_LEAGUE_NORWAY", "TXT_ROW_LEAGUE"));
      await pending.promise;
    });
    // The obsolete result never shows: the inputs it describes are not the current ones.
    expect(shownPreview()).toBeNull();
    // A new request for the current inputs is shown.
    handlers.dictionary_rename_preview = ({ key, donor }) => preview(key, donor, "Sweden text");
    await waitFor(() => expect((screen.getByText("Preview") as HTMLButtonElement).disabled).toBe(false));
    fireEvent.click(screen.getByText("Preview"));
    await waitFor(() => expect(shownPreview()).toBeTruthy());
    expect(screen.getByText("Sweden text")).toBeTruthy();
    const previews = calls.filter((c) => c.cmd === "dictionary_rename_preview");
    const last = previews[previews.length - 1];
    expect(last.args.key).toBe("TXT_LEAGUE_SWEDEN");
  });

  it("drops the preview when the attached dictionary's identity changes", async () => {
    handlers.dictionary_rename_preview = ({ key, donor }) => preview(key, donor);
    const r = mount({ dictionarySha256: "d1" });
    fireEvent.click(screen.getByText("Preview"));
    await waitFor(() => expect(shownPreview()).toBeTruthy());
    r.rerenderWith({ dictionarySha256: "d2" });
    await waitFor(() => expect(shownPreview()).toBeNull());
  });

  it("never publishes a late summary for a project that was replaced while 'Hold rename' was pending", async () => {
    const pendingSet = deferred<DictionaryRepointPreviewDto>();
    handlers.dictionary_rename_set = () => pendingSet.promise;
    const pendingSummary = deferred<ProjectSummaryDto>();
    handlers.project_summary = ({ projectId }) => (projectId === 1 ? pendingSummary.promise : summary({ project_id: projectId }));
    const update = vi.fn();
    projectState = { update };
    const r = mount({ projectId: 1 });
    fireEvent.click(screen.getByText("Hold rename"));
    await waitFor(() => expect(calls.some((c) => c.cmd === "dictionary_rename_set")).toBe(true));
    // Project 1 is closed and project 2 opened while the set is in flight.
    r.rerenderWith({ projectId: 2 });
    await act(async () => {
      pendingSet.resolve(preview("TXT_LEAGUE_NORWAY", "TXT_ROW_LEAGUE"));
      await pendingSet.promise;
    });
    await waitFor(() => expect(calls.some((c) => c.cmd === "project_summary" && c.args.projectId === 1)).toBe(true));
    await act(async () => {
      pendingSummary.resolve(summary({ project_id: 1, workspace_revision: "rev-old-project" }));
      await pendingSummary.promise;
    });
    expect(update).not.toHaveBeenCalled();
    expect(shownPreview()).toBeNull();
    // Project 2's own requests still publish its own summary.
    handlers.dictionary_rename_set = ({ key, donor }) => preview(key, donor);
    await waitFor(() => expect((screen.getByText("Hold rename") as HTMLButtonElement).disabled).toBe(false));
    fireEvent.click(screen.getByText("Hold rename"));
    await waitFor(() => expect(update).toHaveBeenCalled());
    expect(update.mock.calls.every(([s]) => (s as ProjectSummaryDto).project_id === 2)).toBe(true);
    const sets = calls.filter((c) => c.cmd === "dictionary_rename_set");
    expect(sets[sets.length - 1].args.projectId).toBe(2);
  });

  it("publishes nothing after unmount, even when the summary request had already started", async () => {
    handlers.dictionary_rename_clear = () => null;
    const pendingSummary = deferred<ProjectSummaryDto>();
    handlers.project_summary = () => pendingSummary.promise;
    const update = vi.fn();
    projectState = { update };
    const r = mount();
    fireEvent.click(screen.getByText("Clear pending rename"));
    await waitFor(() => expect(calls.some((c) => c.cmd === "project_summary")).toBe(true));
    r.unmount();
    await act(async () => {
      pendingSummary.resolve(summary({ project_id: 1 }));
      await pendingSummary.promise;
    });
    expect(update).not.toHaveBeenCalled();
  });
});

describe("DictionaryRenamePanel growth (LOC-W3)", () => {
  const growthPreview = (over: Record<string, unknown> = {}) => ({
    dictionary_path: "C:/copy/USRDIR/fe/loc/nhl_eng_us.db",
    dictionary_sha256: "d1",
    endian_big: true,
    order_class: "A",
    descents: [],
    studio_touched: false,
    rows_before: 1000,
    rows_after: 1000,
    appended_bytes: 12,
    inserts: [],
    edits: [{ row: { record: 10, key: "TXT_LEAGUE_NORWAY", hash: 1, text: "Norway" }, lookup: "exact", text_before: "Norway", text_after: "Kingdom of Norway", sharers: [], all_sharers: false, records: [10] }],
    warnings: ["This grows the dictionary: 0 row(s) inserted, 1 text(s) appended (12 bytes)"],
    serializer: "preserving",
    new_symbols: [],
    rebuild_accounting: [],
    ...over,
  });

  it("appends text through the growth command with a text op, and inserts a new key with an insert op", async () => {
    handlers.dictionary_growth_preview = ({ ops }) =>
      ops[0].kind === "insert"
        ? growthPreview({ rows_after: 1001, edits: [], inserts: [{ key: ops[0].key, hash: 305419896, text: ops[0].text, position: 500, output_record: 500 }] })
        : growthPreview();
    handlers.dictionary_growth_set = ({ ops }) => growthPreview({ warnings: [`held ${ops[0].kind}`] });
    mount();
    fireEvent.click(screen.getByTestId("mode-grow"));
    fireEvent.change(screen.getByLabelText("new text"), { target: { value: "Kingdom of Norway" } });
    fireEvent.click(screen.getByText("Preview"));
    await waitFor(() => expect(shownPreview()).toBeTruthy());
    expect(calls[calls.length - 1]).toEqual({ cmd: "dictionary_growth_preview", args: { projectId: 1, ops: [{ kind: "text", key: "TXT_LEAGUE_NORWAY", text: "Kingdom of Norway", all_sharers: false }], options: null } });
    expect(shownPreview()!.textContent).toContain("Kingdom of Norway");
    expect(shownPreview()!.textContent).toContain("12 bytes appended");
    expect(shownPreview()!.textContent).toContain("order class A");
    fireEvent.click(screen.getByText("Hold growth"));
    await waitFor(() => expect(calls.some((c) => c.cmd === "dictionary_growth_set")).toBe(true));
    await waitFor(() => expect(shownPreview()!.textContent).toContain("held text"));
    // New key mode: the donor field is the text; an empty text is a null-text key (allowed).
    fireEvent.click(screen.getByTestId("mode-insert"));
    expect(shownPreview()).toBeNull();
    fireEvent.change(keyInput(), { target: { value: "TXT_LEAGUE_STUDIO_NEW" } });
    fireEvent.change(screen.getByLabelText("new text"), { target: { value: "Studio League" } });
    fireEvent.click(screen.getByText("Preview"));
    await waitFor(() => expect(shownPreview()).toBeTruthy());
    expect(calls[calls.length - 1]).toEqual({ cmd: "dictionary_growth_preview", args: { projectId: 1, ops: [{ kind: "insert", key: "TXT_LEAGUE_STUDIO_NEW", text: "Studio League", all_sharers: false }], options: null } });
    expect(shownPreview()!.textContent).toContain("inserted at row 500");
    expect(shownPreview()!.textContent).toContain("rows 1000 → 1001");
  });

  it("shows a class-B refusal as the server states it", async () => {
    handlers.dictionary_growth_preview = () => {
      throw new Error("baseline-order-anomalous: this dictionary's rows are not in hash order (descents at records 183, 482); insertion is not offered on it (text changes are)");
    };
    mount();
    fireEvent.click(screen.getByTestId("mode-insert"));
    fireEvent.change(screen.getByLabelText("new text"), { target: { value: "X" } });
    fireEvent.click(screen.getByText("Preview"));
    await waitFor(() => expect(screen.getByText(/baseline-order-anomalous/)).toBeTruthy());
    expect(shownPreview()).toBeNull();
  });

  it("offers the canonical rebuild after a needs-rebuild refusal and sends it only when turned on, showing its accounting", async () => {
    handlers.dictionary_growth_preview = ({ options }) => {
      if (!options) {
        throw new Error("needs-rebuild: this dictionary's Huffman tree has no symbol for '~' (in \"Norway ~\"); adding one means rebuilding the whole pool");
      }
      return growthPreview({ serializer: "canonical-rebuild", new_symbols: ["~"], appended_bytes: 0, edits: [{ row: { record: 10, key: "TXT_LEAGUE_NORWAY", hash: 1, text: "Norway" }, lookup: "exact", text_before: "Norway", text_after: "Norway ~", sharers: [], all_sharers: false, records: [10] }], warnings: ["This REBUILDS the dictionary"] });
    };
    handlers.dictionary_growth_set = ({ options }) =>
      growthPreview({ serializer: options ? "canonical-rebuild" : "preserving", new_symbols: ["~"], rebuild_accounting: ["compression tree rebuilt: 660 -> 664 bytes", "string pool repacked: 1000 -> 1004 bytes; 0 string(s) moved; 1 record(s) re-pointed; 0 inserted"], warnings: ["held"] });
    mount();
    fireEvent.click(screen.getByTestId("mode-grow"));
    fireEvent.change(screen.getByLabelText("new text"), { target: { value: "Norway ~" } });
    fireEvent.click(screen.getByText("Preview"));
    await waitFor(() => expect(screen.getByText(/needs-rebuild/)).toBeTruthy());
    expect(shownPreview()).toBeNull();
    expect((screen.getByTestId("growth-rebuild") as HTMLInputElement).checked).toBe(false);
    // The offer turns the rebuild on; the next preview sends the serializer explicitly.
    fireEvent.click(screen.getByTestId("growth-offer-rebuild"));
    expect((screen.getByTestId("growth-rebuild") as HTMLInputElement).checked).toBe(true);
    fireEvent.click(screen.getByText("Preview"));
    await waitFor(() => expect(shownPreview()).toBeTruthy());
    expect(calls[calls.length - 1]).toEqual({ cmd: "dictionary_growth_preview", args: { projectId: 1, ops: [{ kind: "text", key: "TXT_LEAGUE_NORWAY", text: "Norway ~", all_sharers: false }], options: { serializer: "rebuild" } } });
    expect(screen.getByTestId("growth-rebuild-facts").textContent).toContain('"~"');
    expect(shownPreview()!.textContent).toContain("canonical rebuild");
    fireEvent.click(screen.getByText("Hold growth"));
    await waitFor(() => expect(calls.some((c) => c.cmd === "dictionary_growth_set")).toBe(true));
    expect(calls.find((c) => c.cmd === "dictionary_growth_set")!.args.options).toEqual({ serializer: "rebuild" });
    await waitFor(() => expect(screen.getByTestId("growth-rebuild-facts").textContent).toContain("compression tree rebuilt"));
    // Turning the rebuild off drops the held preview (it no longer describes what a request returns).
    fireEvent.click(screen.getByTestId("growth-rebuild"));
    expect(shownPreview()).toBeNull();
  });
});
