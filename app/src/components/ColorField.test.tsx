// @vitest-environment jsdom
//
// C8 (overnight 2026-09-20, Phase 2.1 team identity) — the team colour cell: a packed word (PS4
// NHL 17–21) and an `_r/_g/_b` triple (NHL 11–15 / Legacy / PS4 15–16) both show as one swatch +
// hex box, and a commit writes every component through ONE `set_cells_int` call with the exact
// component values; invalid hex and an unchanged value never dispatch. Component evidence only —
// the byte order `(r << 16) | (g << 8) | b` is the domain's measured fact (field_profile.rs).
//
// R1 F2 (20 Sep 2026): the draft and the mutation are bound to their target — project, table,
// record, component fields. A target switch with an EQUAL stored colour drops the draft and the
// error; an in-flight write completes with the arguments it was dispatched with and refreshes the
// project it was dispatched for (also after unmount); Enter + blur (and blur + blur before the
// refetch lands) dispatch once. Assertions are on dispatched arguments and the shared `update`
// calls, not only on the box text. No desktop reproduction is claimed.
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { FieldViewDto } from "../bindings/FieldViewDto";

const calls: Array<{ cmd: string; args: any }> = [];
/** Commands whose promise is held until the test resolves it. */
const gated = new Set<string>();
const held: Array<{ cmd: string; args: any; resolve: (v: unknown) => void; reject: (e: unknown) => void }> = [];
const failing = new Set<string>();
vi.mock("../lib/client", () => ({
  call: (cmd: string, args: any) => {
    calls.push({ cmd, args });
    if (gated.has(cmd)) return new Promise((resolve, reject) => held.push({ cmd, args, resolve, reject }));
    if (failing.has(cmd)) return Promise.reject(new Error(`refused ${cmd}`));
    if (cmd === "project_summary") return Promise.resolve({ project_id: args.projectId, dirty: true });
    return Promise.resolve({ ok: true });
  },
}));
const updates: unknown[] = [];
vi.mock("../lib/project", () => ({
  useProject: () => ({ update: (s: unknown) => updates.push(s) }),
}));

import { ColorField, hexOf, targetKey, writesFor } from "./ColorField";

function fv(field: string, decoded: number, over: Partial<FieldViewDto> = {}): FieldViewDto {
  return {
    studio_table: "kOtt",
    studio_field: field,
    name: field,
    kind: "int",
    raw: decoded,
    decoded,
    display: String(decoded),
    enum_label: null,
    enum_options: [],
    edit_offset: null,
    edit_height: false,
    edit_scale: null,
    range_low: 0,
    range_high: 255,
    editable: true,
    is_string: false,
    ...over,
  } as unknown as FieldViewDto;
}

type Target = { projectId?: number; record?: number; components: FieldViewDto[] };

function tree(qc: QueryClient, t: Target) {
  return (
    <QueryClientProvider client={qc}>
      <ColorField projectId={t.projectId ?? 7} record={t.record ?? 3} components={t.components} label="Primary" />
    </QueryClientProvider>
  );
}

function mountAt(t: Target) {
  const qc = new QueryClient();
  const r = render(tree(qc, t));
  return { ...r, retarget: (n: Target) => r.rerender(tree(qc, n)) };
}

function mount(components: FieldViewDto[]) {
  return mountAt({ components });
}

const box = () => screen.getByTestId("color-hex") as HTMLInputElement;
const writes = () => calls.filter((c) => c.cmd === "set_cells_int");
const summaries = () => calls.filter((c) => c.cmd === "project_summary").map((c) => c.args);
const packed = (v = 0xdd9b26) => [fv("feprimarycolor", v, { range_high: 0xffffff })];
/** Let a queued mutation (if any) reach the mocked client: `mutate` dispatches asynchronously. */
const settle = () => new Promise((r) => setTimeout(r, 0));

describe("ColorField", () => {
  beforeEach(() => {
    calls.length = 0;
    updates.length = 0;
    held.length = 0;
    gated.clear();
    failing.clear();
  });
  afterEach(cleanup);

  it("decodes both shapes to the same hex and encodes them back component-wise", () => {
    expect(hexOf([fv("feprimarycolor", 0xdd9b26)])).toBe("#dd9b26");
    expect(hexOf([fv("feprimarycolor_r", 0xdd), fv("feprimarycolor_g", 0x9b), fv("feprimarycolor_b", 0x26)])).toBe("#dd9b26");
    expect(writesFor([fv("feprimarycolor", 0)], "#dd9b26")).toEqual([{ field: "feprimarycolor", value: 0xdd9b26 }]);
    expect(writesFor([fv("r", 0), fv("g", 0), fv("b", 0)], "#dd9b26")).toEqual([
      { field: "r", value: 0xdd },
      { field: "g", value: 0x9b },
      { field: "b", value: 0x26 },
    ]);
  });

  it("writes a triple as one set_cells_int call with the three byte values", async () => {
    mount([fv("primarycolor_r", 0xff), fv("primarycolor_g", 0xad), fv("primarycolor_b", 0x0f)]);
    expect(box().value).toBe("#ffad0f");
    fireEvent.change(box(), { target: { value: "0A1B2C" } });
    fireEvent.keyDown(box(), { key: "Enter" });
    await waitFor(() => expect(summaries()).toHaveLength(1));
    expect(writes()).toHaveLength(1);
    expect(writes()[0].args).toEqual({
      projectId: 7,
      table: "kOtt",
      record: 3,
      writes: [
        { field: "primarycolor_r", value: 0x0a },
        { field: "primarycolor_g", value: 0x1b },
        { field: "primarycolor_b", value: 0x2c },
      ],
    });
    await waitFor(() => expect(updates).toEqual([{ project_id: 7, dirty: true }]));
  });

  it("writes a packed word as one value and ignores invalid or unchanged input", async () => {
    mount(packed());
    fireEvent.change(box(), { target: { value: "#12" } });
    fireEvent.blur(box());
    expect(screen.getByText("six hex digits")).toBeTruthy();
    fireEvent.change(box(), { target: { value: "#DD9B26" } });
    fireEvent.blur(box());
    await settle();
    expect(writes()).toHaveLength(0);
    fireEvent.change(box(), { target: { value: "#000000" } });
    fireEvent.blur(box());
    await waitFor(() => expect(writes()).toHaveLength(1));
    expect(writes()[0].args.writes).toEqual([{ field: "feprimarycolor", value: 0 }]);
  });

  it("is read-only when a component is not editable", () => {
    mount([fv("primarycolor_r", 1), fv("primarycolor_g", 2, { editable: false }), fv("primarycolor_b", 3)]);
    expect(box().disabled).toBe(true);
  });

  // ---------------------------------------------------------------- R1 F2: target binding

  it("keys the draft on project, table, record and fields", () => {
    expect(targetKey(7, 3, packed())).toBe("7/kOtt/3/feprimarycolor");
    expect(targetKey(7, 3, [fv("r", 1), fv("g", 2), fv("b", 3)])).toBe("7/kOtt/3/r+g+b");
    expect(targetKey(7, 3, packed())).not.toBe(targetKey(7, 8, packed()));
    expect(targetKey(7, 3, packed())).not.toBe(targetKey(9, 3, packed()));
    expect(targetKey(7, 3, packed())).not.toBe(targetKey(7, 3, [fv("fesecondarycolor", 0xdd9b26)]));
  });

  it("drops an uncommitted draft when the record changes to a team with the same colour (the reviewer's reproduction)", async () => {
    const m = mountAt({ record: 3, components: packed() });
    fireEvent.change(box(), { target: { value: "#123456" } });
    expect(box().value).toBe("#123456");
    // Team 8 stores the identical colour: the draft must not survive into it.
    m.retarget({ record: 8, components: packed() });
    expect(box().value).toBe("#dd9b26");
    fireEvent.blur(box());
    await settle();
    expect(writes()).toHaveLength(0);
    // A commit made on team 8 goes to team 8 with its own value.
    fireEvent.change(box(), { target: { value: "#abcdef" } });
    fireEvent.blur(box());
    await waitFor(() => expect(writes()).toHaveLength(1));
    expect(writes()[0].args).toEqual({ projectId: 7, table: "kOtt", record: 8, writes: [{ field: "feprimarycolor", value: 0xabcdef }] });
    await waitFor(() => expect(updates).toEqual([{ project_id: 7, dirty: true }]));
  });

  it("drops the draft when the project changes with the same colour, and writes to the new project only on a new commit", async () => {
    const m = mountAt({ projectId: 7, components: packed() });
    fireEvent.change(box(), { target: { value: "#123456" } });
    m.retarget({ projectId: 9, components: packed() });
    expect(box().value).toBe("#dd9b26");
    fireEvent.keyDown(box(), { key: "Enter" });
    fireEvent.blur(box());
    await settle();
    expect(writes()).toHaveLength(0);
    fireEvent.change(box(), { target: { value: "#010203" } });
    fireEvent.keyDown(box(), { key: "Enter" });
    await waitFor(() => expect(writes()).toHaveLength(1));
    expect(writes()[0].args.projectId).toBe(9);
    await waitFor(() => expect(updates).toEqual([{ project_id: 9, dirty: true }]));
    expect(summaries()).toEqual([{ projectId: 9 }]);
  });

  it("drops the draft when the component fields change with the same colour, and keeps it when nothing changed", async () => {
    const m = mountAt({ components: packed() });
    fireEvent.change(box(), { target: { value: "#123456" } });
    m.retarget({ components: [fv("fesecondarycolor", 0xdd9b26, { range_high: 0xffffff })] });
    expect(box().value).toBe("#dd9b26");
    fireEvent.blur(box());
    await settle();
    expect(writes()).toHaveLength(0);
    // Same fields, same colour, same record and project: a re-render keeps the draft.
    fireEvent.change(box(), { target: { value: "#654321" } });
    m.retarget({ components: [fv("fesecondarycolor", 0xdd9b26, { range_high: 0xffffff })] });
    expect(box().value).toBe("#654321");
  });

  it("completes an in-flight write with the arguments it was dispatched with after the target changed", async () => {
    gated.add("set_cells_int");
    const m = mountAt({ record: 3, components: packed() });
    fireEvent.change(box(), { target: { value: "#123456" } });
    fireEvent.blur(box());
    await waitFor(() => expect(held).toHaveLength(1));
    expect(held[0].args).toEqual({ projectId: 7, table: "kOtt", record: 3, writes: [{ field: "feprimarycolor", value: 0x123456 }] });
    await waitFor(() => expect(box().disabled).toBe(true));
    // The user moves to team 8 (same colour) while the write is in flight: the new editor is its
    // own — enabled, showing team 8's stored colour, no draft, no pending state.
    m.retarget({ record: 8, components: packed() });
    expect(box().disabled).toBe(false);
    expect(box().value).toBe("#dd9b26");
    held[0].resolve({ ok: true });
    await waitFor(() => expect(updates).toHaveLength(1));
    expect(summaries()).toEqual([{ projectId: 7 }]);
    expect(updates).toEqual([{ project_id: 7, dirty: true }]);
    // Exactly one write, still the original one; a blur on team 8 adds nothing.
    fireEvent.blur(box());
    expect(writes()).toHaveLength(1);
    expect(writes()[0].args.record).toBe(3);
  });

  it("completes an in-flight write for its own project after unmount", async () => {
    gated.add("set_cells_int");
    const m = mountAt({ projectId: 7, record: 3, components: packed() });
    fireEvent.change(box(), { target: { value: "#123456" } });
    fireEvent.keyDown(box(), { key: "Enter" });
    await waitFor(() => expect(held).toHaveLength(1));
    m.unmount();
    held[0].resolve({ ok: true });
    await waitFor(() => expect(updates).toHaveLength(1));
    expect(summaries()).toEqual([{ projectId: 7 }]);
    expect(updates).toEqual([{ project_id: 7, dirty: true }]);
    expect(writes()).toHaveLength(1);
    expect(writes()[0].args).toEqual({ projectId: 7, table: "kOtt", record: 3, writes: [{ field: "feprimarycolor", value: 0x123456 }] });
  });

  it("dispatches once for Enter followed by blur, and once for repeated blurs before the refetch lands", async () => {
    gated.add("set_cells_int");
    mount(packed());
    fireEvent.change(box(), { target: { value: "#123456" } });
    fireEvent.keyDown(box(), { key: "Enter" });
    fireEvent.blur(box());
    fireEvent.blur(box());
    await waitFor(() => expect(held).toHaveLength(1));
    expect(writes()).toHaveLength(1);
    held[0].resolve({ ok: true });
    await waitFor(() => expect(updates).toHaveLength(1));
    // The stored colour has not been refetched yet (props unchanged): the same value must not be
    // sent again, but a different one is a new write.
    fireEvent.blur(box());
    expect(writes()).toHaveLength(1);
    fireEvent.change(box(), { target: { value: "#654321" } });
    fireEvent.blur(box());
    await waitFor(() => expect(writes()).toHaveLength(2));
    expect(writes()[1].args.writes).toEqual([{ field: "feprimarycolor", value: 0x654321 }]);
  });

  it("shows a refusal, allows a retry of the same value, and clears the refusal on a target switch with the same colour", async () => {
    failing.add("set_cells_int");
    const m = mountAt({ record: 3, components: packed() });
    fireEvent.change(box(), { target: { value: "#123456" } });
    fireEvent.blur(box());
    await waitFor(() => expect(screen.getByText(/refused set_cells_int/)).toBeTruthy());
    expect(updates).toHaveLength(0);
    // Retry after a refusal re-dispatches (the in-flight guard was released).
    fireEvent.blur(box());
    await waitFor(() => expect(writes()).toHaveLength(2));
    expect(writes().every((w) => w.args.record === 3)).toBe(true);
    // Team 8, same colour: no error, no draft.
    m.retarget({ record: 8, components: packed() });
    expect(screen.queryByText(/refused set_cells_int/)).toBeNull();
    expect(box().value).toBe("#dd9b26");
  });

  it("re-seeds the draft when the stored colour of the same target changes (refetch after commit, undo)", () => {
    const m = mountAt({ components: packed(0x111111) });
    expect(box().value).toBe("#111111");
    m.retarget({ components: packed(0x222222) });
    expect(box().value).toBe("#222222");
    expect(screen.getByTestId("color-swatch").getAttribute("title")).toBe("#222222");
  });
  // ---------------------------------------------------------------- R2 F1: acknowledgement + reapply

  it("reapplies a value after edit → acknowledgement → undo (the reviewer's reproduction)", async () => {
    const m = mountAt({ components: packed() });
    fireEvent.change(box(), { target: { value: "#123456" } });
    fireEvent.blur(box());
    await waitFor(() => expect(updates).toHaveLength(1));
    await waitFor(() => expect(box().disabled).toBe(false));
    expect(writes()).toHaveLength(1);
    // Acknowledgement: the stored colour now reads the edit.
    m.retarget({ components: packed(0x123456) });
    expect(box().value).toBe("#123456");
    fireEvent.blur(box());
    await settle();
    expect(writes()).toHaveLength(1);
    // Undo: the stored colour goes back.
    m.retarget({ components: packed(0xdd9b26) });
    expect(box().value).toBe("#dd9b26");
    // Reapply the same edit: a genuine second write.
    fireEvent.change(box(), { target: { value: "#123456" } });
    fireEvent.blur(box());
    await waitFor(() => expect(writes()).toHaveLength(2));
    expect(writes()[1].args).toEqual({ projectId: 7, table: "kOtt", record: 3, writes: [{ field: "feprimarycolor", value: 0x123456 }] });
    await waitFor(() => expect(updates).toHaveLength(2));
    expect(box().value).toBe("#123456");
  });

  it("releases the post-success suppression on any authoritative change of the same target", async () => {
    const m = mountAt({ components: packed() });
    fireEvent.change(box(), { target: { value: "#123456" } });
    fireEvent.keyDown(box(), { key: "Enter" });
    await waitFor(() => expect(updates).toHaveLength(1));
    // Still awaiting acknowledgement: the same hex is suppressed.
    fireEvent.blur(box());
    await settle();
    expect(writes()).toHaveLength(1);
    // The authoritative value changes to something else (a reload / another edit): the box shows
    // it and the earlier hex is a new edit again.
    m.retarget({ components: packed(0x0f0f0f) });
    expect(box().value).toBe("#0f0f0f");
    fireEvent.change(box(), { target: { value: "#123456" } });
    fireEvent.blur(box());
    await waitFor(() => expect(writes()).toHaveLength(2));
    expect(writes()[1].args.writes).toEqual([{ field: "feprimarycolor", value: 0x123456 }]);
    // Acknowledged: the same hex again is unchanged, not a write.
    m.retarget({ components: packed(0x123456) });
    fireEvent.keyDown(box(), { key: "Enter" });
    fireEvent.blur(box());
    await settle();
    expect(writes()).toHaveLength(2);
    expect(box().value).toBe("#123456");
  });

  it("holds a draft changed while a request is pending and commits it once the request settled", async () => {
    gated.add("set_cells_int");
    mount(packed());
    fireEvent.change(box(), { target: { value: "#123456" } });
    fireEvent.blur(box());
    await waitFor(() => expect(held).toHaveLength(1));
    // A different value typed while the write is pending: kept in the box, not dispatched.
    fireEvent.change(box(), { target: { value: "#0c0c0c" } });
    fireEvent.keyDown(box(), { key: "Enter" });
    fireEvent.blur(box());
    await settle();
    expect(writes()).toHaveLength(1);
    expect(box().value).toBe("#0c0c0c");
    held[0].resolve({ ok: true });
    await waitFor(() => expect(updates).toHaveLength(1));
    await waitFor(() => expect(box().disabled).toBe(false));
    expect(box().value).toBe("#0c0c0c");
    // Now it commits as its own write.
    fireEvent.blur(box());
    await waitFor(() => expect(held).toHaveLength(2));
    expect(held[1].args.writes).toEqual([{ field: "feprimarycolor", value: 0x0c0c0c }]);
    held[1].resolve({ ok: true });
    await waitFor(() => expect(updates).toHaveLength(2));
    expect(writes()).toHaveLength(2);
    expect(box().value).toBe("#0c0c0c");
  });

  it("retries the same value after a failed write, and a later success is then acknowledged normally", async () => {
    failing.add("set_cells_int");
    const m = mountAt({ components: packed() });
    fireEvent.change(box(), { target: { value: "#123456" } });
    fireEvent.keyDown(box(), { key: "Enter" });
    await waitFor(() => expect(screen.getByText(/refused set_cells_int/)).toBeTruthy());
    expect(writes()).toHaveLength(1);
    expect(updates).toHaveLength(0);
    // Retry: dispatched again with the same args (nothing was acknowledged).
    fireEvent.blur(box());
    await waitFor(() => expect(writes()).toHaveLength(2));
    expect(writes()[1].args).toEqual(writes()[0].args);
    // The backend recovers: the third attempt succeeds, a fourth Enter is a duplicate.
    failing.clear();
    await waitFor(() => expect(box().disabled).toBe(false));
    fireEvent.keyDown(box(), { key: "Enter" });
    await waitFor(() => expect(updates).toHaveLength(1));
    fireEvent.keyDown(box(), { key: "Enter" });
    await settle();
    expect(writes()).toHaveLength(3);
    m.retarget({ components: packed(0x123456) });
    expect(box().value).toBe("#123456");
    expect(screen.queryByText(/refused set_cells_int/)).toBeNull();
  });

  // ---------------------------------------------------------------- R3 F1: settlement ordering
  //
  // Settlement (`onSettled`) runs only after the invalidation and the owning project's summary
  // refresh; with the summary response held, the authoritative value can change — acknowledgement,
  // undo, a reload to a third value — BEFORE the write settles. The duplicate guard armed at
  // settlement must be bound to the state the write was dispatched against: a late completion
  // never reinstates a guard that newer authoritative state already released.

  /** Dispatch `hex` with the summary response held; the write itself succeeds immediately. */
  async function dispatchHoldingSummary(hex: string) {
    gated.add("project_summary");
    fireEvent.change(box(), { target: { value: hex } });
    fireEvent.blur(box());
    await waitFor(() => expect(held.filter((h) => h.cmd === "project_summary")).toHaveLength(1));
    expect(box().disabled).toBe(true);
  }

  /** Resolve the held summary for project 7 and wait for the editor to re-enable. */
  async function resolveSummaryAndSettle() {
    const h = held.find((x) => x.cmd === "project_summary")!;
    held.splice(held.indexOf(h), 1);
    const before = updates.length;
    h.resolve({ project_id: 7, dirty: false });
    await waitFor(() => expect(updates).toHaveLength(before + 1));
    await waitFor(() => expect(box().disabled).toBe(false));
  }

  it("does not re-arm the guard when the acknowledgement arrived before settlement", async () => {
    const m = mountAt({ components: packed() });
    await dispatchHoldingSummary("#123456");
    expect(writes()).toHaveLength(1);
    // Acknowledgement while the summary is still pending.
    m.retarget({ components: packed(0x123456) });
    expect(box().value).toBe("#123456");
    await resolveSummaryAndSettle();
    expect(updates).toEqual([{ project_id: 7, dirty: false }]);
    // Same value after the acknowledgement: unchanged, not a write.
    fireEvent.keyDown(box(), { key: "Enter" });
    fireEvent.blur(box());
    await settle();
    expect(writes()).toHaveLength(1);
    // A reload to a third value, then the original edit again: a genuine second write — the late
    // settlement must not have reinstated "#123456 awaiting acknowledgement".
    m.retarget({ components: packed(0x0f0f0f) });
    fireEvent.change(box(), { target: { value: "#123456" } });
    fireEvent.blur(box());
    await waitFor(() => expect(writes()).toHaveLength(2));
    expect(writes()[1].args).toEqual({ projectId: 7, table: "kOtt", record: 3, writes: [{ field: "feprimarycolor", value: 0x123456 }] });
  });

  it("writes again after acknowledgement and undo both arrived before settlement (the R3 reproduction)", async () => {
    const m = mountAt({ components: packed() });
    await dispatchHoldingSummary("#123456");
    expect(writes()).toHaveLength(1);
    // Authoritative B (acknowledgement), then authoritative A (undo), while the summary is held.
    m.retarget({ components: packed(0x123456) });
    expect(box().value).toBe("#123456");
    m.retarget({ components: packed(0xdd9b26) });
    expect(box().value).toBe("#dd9b26");
    await resolveSummaryAndSettle();
    // Enter B again: expected a second write (R2 dispatched only once here).
    fireEvent.change(box(), { target: { value: "#123456" } });
    fireEvent.keyDown(box(), { key: "Enter" });
    await waitFor(() => expect(writes()).toHaveLength(2));
    expect(writes()[1].args).toEqual({ projectId: 7, table: "kOtt", record: 3, writes: [{ field: "feprimarycolor", value: 0x123456 }] });
    // ... and that second write is itself a single dispatch under Enter + blur.
    fireEvent.blur(box());
    await settle();
    expect(writes()).toHaveLength(2);
    expect(summaries()).toEqual([{ projectId: 7 }, { projectId: 7 }]);
  });

  it("treats the edit as new when a different authoritative value arrived before settlement", async () => {
    const m = mountAt({ components: packed() });
    await dispatchHoldingSummary("#123456");
    // Not the acknowledgement: another client's edit / a reload to a third value.
    m.retarget({ components: packed(0x0f0f0f) });
    expect(box().value).toBe("#0f0f0f");
    await resolveSummaryAndSettle();
    fireEvent.change(box(), { target: { value: "#123456" } });
    fireEvent.blur(box());
    await waitFor(() => expect(writes()).toHaveLength(2));
    expect(writes()[1].args.writes).toEqual([{ field: "feprimarycolor", value: 0x123456 }]);
  });

  it("keeps ordinary suppression when the acknowledgement arrives after settlement", async () => {
    const m = mountAt({ components: packed() });
    await dispatchHoldingSummary("#123456");
    await resolveSummaryAndSettle();
    // Settled, not yet acknowledged: the same hex is a duplicate.
    fireEvent.blur(box());
    fireEvent.keyDown(box(), { key: "Enter" });
    await settle();
    expect(writes()).toHaveLength(1);
    // Acknowledged: still unchanged.
    m.retarget({ components: packed(0x123456) });
    fireEvent.keyDown(box(), { key: "Enter" });
    await settle();
    expect(writes()).toHaveLength(1);
    // Undo: the same hex is a genuine new write.
    m.retarget({ components: packed(0xdd9b26) });
    fireEvent.change(box(), { target: { value: "#123456" } });
    fireEvent.blur(box());
    await waitFor(() => expect(writes()).toHaveLength(2));
  });

  it("dispatches once for Enter/blur duplicates while the summary is held and nothing acknowledged", async () => {
    mountAt({ components: packed() });
    await dispatchHoldingSummary("#123456");
    fireEvent.blur(box());
    fireEvent.keyDown(box(), { key: "Enter" });
    fireEvent.blur(box());
    await settle();
    expect(writes()).toHaveLength(1);
    await resolveSummaryAndSettle();
    // Settled, still not acknowledged (props unchanged): still a duplicate.
    fireEvent.blur(box());
    fireEvent.keyDown(box(), { key: "Enter" });
    await settle();
    expect(writes()).toHaveLength(1);
    // A different value is a new write.
    fireEvent.change(box(), { target: { value: "#654321" } });
    fireEvent.blur(box());
    await waitFor(() => expect(writes()).toHaveLength(2));
  });

  it("retries after a failed write and then settles the successful retry with the summary held", async () => {
    failing.add("set_cells_int");
    const m = mountAt({ components: packed() });
    fireEvent.change(box(), { target: { value: "#123456" } });
    fireEvent.blur(box());
    await waitFor(() => expect(screen.getByText(/refused set_cells_int/)).toBeTruthy());
    expect(writes()).toHaveLength(1);
    expect(summaries()).toHaveLength(0);
    failing.clear();
    gated.add("project_summary");
    fireEvent.blur(box());
    await waitFor(() => expect(writes()).toHaveLength(2));
    expect(writes()[1].args).toEqual(writes()[0].args);
    await waitFor(() => expect(held.filter((h) => h.cmd === "project_summary")).toHaveLength(1));
    // Acknowledged before settlement, then undone: reapply is a third write after settlement.
    m.retarget({ components: packed(0x123456) });
    m.retarget({ components: packed(0xdd9b26) });
    await resolveSummaryAndSettle();
    expect(screen.queryByText(/refused set_cells_int/)).toBeNull();
    fireEvent.change(box(), { target: { value: "#123456" } });
    fireEvent.keyDown(box(), { key: "Enter" });
    await waitFor(() => expect(writes()).toHaveLength(3));
  });
});
