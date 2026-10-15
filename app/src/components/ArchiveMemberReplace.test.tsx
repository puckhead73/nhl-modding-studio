// @vitest-environment jsdom
//
// OVERNIGHT-DEV-R1 F1 — the archive-member replace flow owns its async steps: the picker, the
// preview and the apply are bound to the project / install root and the pick generation they
// started in. Late completions for a retired generation or another context never repaint, never
// fire the caller's callback and never push a foreign summary; a same-project apply adopts the
// returned summary (dirty) and refreshes queries; Apply cannot be dispatched twice; hiding the
// dialog is not offered while an apply is in flight. Exact dispatched arguments are asserted.
// Component evidence only; ticket ownership on the server is the Rust suite's job.
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { AssetMemberPlanDto } from "../bindings/AssetMemberPlanDto";

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
const updates: Array<{ project_id: number; dirty: boolean }> = [];
vi.mock("../lib/project", () => ({
  useProject: () => ({ update: (s: { project_id: number; dirty: boolean }) => updates.push(s) }),
}));
// The picker is a deferred per call so a test can switch context while it is open.
const pickers: Array<Deferred<string | null>> = [];
vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: () => {
    const d = deferred<string | null>();
    pickers.push(d);
    return d.promise;
  },
  save: () => Promise.resolve(null),
}));

import { useArchiveMemberReplace } from "./ArchiveMemberReplace";

const ARCHIVE = "databigs/inxp_10.big";
const L7 = "nhl/ui/kettle_assets/leaguelogos/size256/l7.gnf";
const L8 = "nhl/ui/kettle_assets/leaguelogos/size256/l8.gnf";

function plan(over: Partial<AssetMemberPlanDto> = {}): AssetMemberPlanDto {
  return {
    ticket: "ns/1",
    archive_relative: ARCHIVE,
    member_path: L7,
    container: "gnf",
    width: 256,
    height: 256,
    stored_form: "chunkzip",
    source_stored_len: 100n,
    new_stored_len: 120n,
    shared_with: [],
    scope: "this-member",
    placement_prediction: "fits its slot",
    description: "replace l7",
    paired_loose: null,
    loose_must_be_absent: null,
    notes: [],
    ...over,
  };
}

const onApplied = vi.fn();
function Host({ projectId, installRoot, member = L7 }: { projectId: number; installRoot: string | null; member?: string }) {
  const r = useArchiveMemberReplace(projectId, installRoot, onApplied);
  return (
    <div>
      <button data-testid="pick" onClick={() => void r.pick(ARCHIVE, member, "logo")}>
        pick
      </button>
      {r.dialog}
      {r.result}
    </div>
  );
}

let qc: QueryClient;
let invalidated = 0;
function mount(projectId = 7, installRoot: string | null = "E:/x/nhl21", member = L7) {
  qc = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } });
  const orig = qc.invalidateQueries.bind(qc);
  qc.invalidateQueries = ((...a: any[]) => {
    invalidated += 1;
    return (orig as any)(...a);
  }) as any;
  const view = render(
    <QueryClientProvider client={qc}>
      <Host projectId={projectId} installRoot={installRoot} member={member} />
    </QueryClientProvider>,
  );
  const rerender = (p: number, root: string | null, m = L7) =>
    view.rerender(
      <QueryClientProvider client={qc}>
        <Host projectId={p} installRoot={root} member={m} />
      </QueryClientProvider>,
    );
  return { ...view, rerender };
}

/** Open the picker and resolve it with `path` (the default path when omitted). */
async function pickAndChoose(path: string | null = "C:/tmp/new.png") {
  fireEvent.click(screen.getByTestId("pick"));
  await waitFor(() => expect(pickers.length).toBeGreaterThan(0));
  const d = pickers[pickers.length - 1];
  await act(async () => {
    d.resolve(path);
    await d.promise;
  });
}

beforeEach(() => {
  calls.length = 0;
  updates.length = 0;
  pickers.length = 0;
  invalidated = 0;
  onApplied.mockReset();
  for (const k of Object.keys(handlers)) delete handlers[k];
});
afterEach(() => cleanup());

describe("useArchiveMemberReplace ownership", () => {
  it("previews with the exact context and applies by ticket only; a same-project apply adopts the summary and refreshes", async () => {
    handlers.asset_member_preview = () => ({ plan: plan(), refusal_kind: null, refusal: null });
    handlers.asset_member_apply_review = () => ({ plan: plan(), pending: 1, summary: { project_id: 7, dirty: true } });
    mount();
    await pickAndChoose();
    await screen.findByText(/fits its slot/);
    expect(calls).toEqual([
      { cmd: "asset_member_preview", args: { projectId: 7, installRoot: "E:/x/nhl21", archiveRelative: ARCHIVE, memberPath: L7, sourceImagePath: "C:/tmp/new.png", wholeGroup: false } },
    ]);
    fireEvent.click(screen.getByTestId("archive-member-apply"));
    await screen.findByTestId("archive-member-result");
    expect(calls[1]).toEqual({ cmd: "asset_member_apply_review", args: { projectId: 7, ticket: "ns/1" } });
    expect(calls).toHaveLength(2);
    expect(updates).toEqual([{ project_id: 7, dirty: true }]);
    expect(invalidated).toBe(1);
    expect(onApplied).toHaveBeenCalledTimes(1);
    expect(screen.queryByTestId("archive-member-dialog")).toBeNull();
    expect(screen.getByTestId("archive-member-result").textContent).toContain(L7);
  });

  it("opens nothing when the picker returns after the project changed", async () => {
    handlers.asset_member_preview = () => ({ plan: plan(), refusal_kind: null, refusal: null });
    const view = mount(7);
    fireEvent.click(screen.getByTestId("pick"));
    await waitFor(() => expect(pickers.length).toBe(1));
    view.rerender(9, "E:/x/nhl20");
    await act(async () => {
      pickers[0].resolve("C:/tmp/late.png");
      await pickers[0].promise;
    });
    expect(screen.queryByTestId("archive-member-dialog")).toBeNull();
    expect(calls).toEqual([]);
    // The picker returning after a cancel-free member switch on the same project is retired too:
    // the new member owns the next pick.
    fireEvent.click(screen.getByTestId("pick"));
    await waitFor(() => expect(pickers.length).toBe(2));
    view.rerender(9, "E:/x/nhl20", L8);
    fireEvent.click(screen.getByTestId("pick"));
    await waitFor(() => expect(pickers.length).toBe(3));
    await act(async () => {
      pickers[1].resolve("C:/tmp/stale.png");
      await pickers[1].promise;
    });
    expect(calls).toEqual([]);
    await act(async () => {
      pickers[2].resolve("C:/tmp/fresh.png");
      await pickers[2].promise;
    });
    await screen.findByTestId("archive-member-dialog");
    expect(calls).toEqual([{ cmd: "asset_member_preview", args: { projectId: 9, installRoot: "E:/x/nhl20", archiveRelative: ARCHIVE, memberPath: L8, sourceImagePath: "C:/tmp/fresh.png", wholeGroup: false } }]);
  });

  it("drops a preview that returns after a context switch, a cancel or a newer pick", async () => {
    const first = deferred<any>();
    let n = 0;
    handlers.asset_member_preview = () => (n++ === 0 ? first.promise : { plan: plan({ ticket: "ns/2", member_path: L8, placement_prediction: "second plan" }), refusal_kind: null, refusal: null });
    const view = mount(7);
    await pickAndChoose("C:/tmp/a.png");
    await screen.findByText(/Planning the member rewrite/);
    // Context switch while planning: the dialog closes and the late plan never shows.
    view.rerender(9, "E:/x/nhl20");
    expect(screen.queryByTestId("archive-member-dialog")).toBeNull();
    await act(async () => {
      first.resolve({ plan: plan(), refusal_kind: null, refusal: null });
      await first.promise;
    });
    expect(screen.queryByTestId("archive-member-dialog")).toBeNull();
    expect(screen.queryByText(/fits its slot/)).toBeNull();

    // Cancel while planning: the late plan is dropped.
    const second = deferred<any>();
    handlers.asset_member_preview = () => second.promise;
    await pickAndChoose("C:/tmp/b.png");
    await screen.findByText(/Planning the member rewrite/);
    fireEvent.click(screen.getByTestId("archive-member-cancel"));
    expect(screen.queryByTestId("archive-member-dialog")).toBeNull();
    await act(async () => {
      second.resolve({ plan: plan(), refusal_kind: null, refusal: null });
      await second.promise;
    });
    expect(screen.queryByTestId("archive-member-dialog")).toBeNull();

    // A newer pick supersedes an older in-flight preview: only the newer plan is shown, and it
    // is the one Apply would send.
    const third = deferred<any>();
    let k = 0;
    handlers.asset_member_preview = () => (k++ === 0 ? third.promise : { plan: plan({ ticket: "ns/9", placement_prediction: "newest plan" }), refusal_kind: null, refusal: null });
    await pickAndChoose("C:/tmp/c.png");
    await screen.findByText(/Planning the member rewrite/);
    await pickAndChoose("C:/tmp/d.png");
    await screen.findByText(/newest plan/);
    await act(async () => {
      third.resolve({ plan: plan({ ticket: "ns/8", placement_prediction: "older plan" }), refusal_kind: null, refusal: null });
      await third.promise;
    });
    expect(screen.queryByText(/older plan/)).toBeNull();
    expect(screen.getByTestId("archive-member-dialog").getAttribute("data-ticket")).toBe("ns/9");
  });

  it("neither shows a result, fires the callback nor adopts the summary when an apply completes after a project switch or unmount", async () => {
    handlers.asset_member_preview = () => ({ plan: plan(), refusal_kind: null, refusal: null });
    const late = deferred<any>();
    handlers.asset_member_apply_review = () => late.promise;
    const view = mount(7);
    await pickAndChoose();
    await screen.findByText(/fits its slot/);
    fireEvent.click(screen.getByTestId("archive-member-apply"));
    await waitFor(() => expect(calls.length).toBe(2));
    expect(calls[1]).toEqual({ cmd: "asset_member_apply_review", args: { projectId: 7, ticket: "ns/1" } });
    view.rerender(9, "E:/x/nhl20");
    await act(async () => {
      late.resolve({ plan: plan(), pending: 1, summary: { project_id: 7, dirty: true } });
      await late.promise;
    });
    expect(updates).toEqual([]);
    expect(onApplied).not.toHaveBeenCalled();
    expect(screen.queryByTestId("archive-member-result")).toBeNull();
    expect(screen.queryByTestId("archive-member-dialog")).toBeNull();
    // Queries were still refreshed: the server did apply it.
    expect(invalidated).toBe(1);

    // Unmount during apply.
    handlers.asset_member_preview = () => ({ plan: plan({ ticket: "ns/3" }), refusal_kind: null, refusal: null });
    const late2 = deferred<any>();
    handlers.asset_member_apply_review = () => late2.promise;
    await pickAndChoose();
    await screen.findByText(/fits its slot/);
    fireEvent.click(screen.getByTestId("archive-member-apply"));
    await waitFor(() => expect(calls.length).toBe(4));
    view.unmount();
    await act(async () => {
      late2.resolve({ plan: plan(), pending: 2, summary: { project_id: 9, dirty: true } });
      await late2.promise;
    });
    expect(updates).toEqual([]);
    expect(onApplied).not.toHaveBeenCalled();
  });

  it("adopts the summary but shows nothing when only the install root changed under a same-project apply", async () => {
    handlers.asset_member_preview = () => ({ plan: plan(), refusal_kind: null, refusal: null });
    const late = deferred<any>();
    handlers.asset_member_apply_review = () => late.promise;
    const view = mount(7, "E:/x/nhl21");
    await pickAndChoose();
    await screen.findByText(/fits its slot/);
    fireEvent.click(screen.getByTestId("archive-member-apply"));
    await waitFor(() => expect(calls.length).toBe(2));
    view.rerender(7, "E:/y/other-root");
    await act(async () => {
      late.resolve({ plan: plan(), pending: 1, summary: { project_id: 7, dirty: true } });
      await late.promise;
    });
    // The project is the same one: its dirty state is real and adopted; the result belonged to
    // the old root's member and is not shown.
    expect(updates).toEqual([{ project_id: 7, dirty: true }]);
    expect(onApplied).not.toHaveBeenCalled();
    expect(screen.queryByTestId("archive-member-result")).toBeNull();
  });

  it("dispatches Apply once while one is in flight, keeps the dialog open (Cancel disabled) and lands the completion", async () => {
    handlers.asset_member_preview = () => ({ plan: plan(), refusal_kind: null, refusal: null });
    const slow = deferred<any>();
    handlers.asset_member_apply_review = () => slow.promise;
    mount(7);
    await pickAndChoose();
    await screen.findByText(/fits its slot/);
    const applyButton = screen.getByTestId("archive-member-apply");
    fireEvent.click(applyButton);
    fireEvent.click(applyButton);
    fireEvent.click(applyButton);
    await waitFor(() => expect(screen.getByTestId("archive-member-applying")).toBeTruthy());
    expect(calls.filter((c) => c.cmd === "asset_member_apply_review")).toHaveLength(1);
    expect((applyButton as HTMLButtonElement).disabled).toBe(true);
    // Closing is not offered while the request is out: the dialog stays, the text says why.
    const cancel = screen.getByTestId("archive-member-cancel") as HTMLButtonElement;
    expect(cancel.disabled).toBe(true);
    fireEvent.click(cancel);
    expect(screen.getByTestId("archive-member-dialog")).toBeTruthy();
    expect(screen.getByTestId("archive-member-applying").textContent).toContain("would not cancel it");
    // A new pick is refused while applying (the plan under the apply must not change).
    fireEvent.click(screen.getByTestId("pick"));
    expect(pickers).toHaveLength(1);
    await act(async () => {
      slow.resolve({ plan: plan(), pending: 1, summary: { project_id: 7, dirty: true } });
      await slow.promise;
    });
    await screen.findByTestId("archive-member-result");
    expect(screen.queryByTestId("archive-member-dialog")).toBeNull();
    expect(updates).toEqual([{ project_id: 7, dirty: true }]);
    expect(onApplied).toHaveBeenCalledTimes(1);
    expect(calls.filter((c) => c.cmd === "asset_member_apply_review")).toHaveLength(1);
  });

  it("shows a refused apply as the server states it and drops the held plan", async () => {
    handlers.asset_member_preview = () => ({ plan: plan(), refusal_kind: null, refusal: null });
    handlers.asset_member_apply_review = () => {
      throw new Error("this texture review is no longer held by the project");
    };
    mount(7);
    await pickAndChoose();
    await screen.findByText(/fits its slot/);
    fireEvent.click(screen.getByTestId("archive-member-apply"));
    await screen.findByText(/no longer held/);
    expect((screen.getByTestId("archive-member-apply") as HTMLButtonElement).disabled).toBe(true);
    expect(updates).toEqual([]);
    expect(onApplied).not.toHaveBeenCalled();
    expect(screen.queryByTestId("archive-member-result")).toBeNull();
  });

  it("offers the scope of a shared block, re-previews with the chosen scope and applies that plan (C7)", async () => {
    handlers.asset_member_preview = ({ wholeGroup }) => ({
      plan: plan({
        ticket: wholeGroup ? "ns/2" : "ns/1",
        shared_with: [L8],
        scope: wholeGroup ? "whole-group" : "this-member",
        placement_prediction: wholeGroup ? "whole group: fits the shared block" : "private copy appended",
      }),
      refusal_kind: null,
      refusal: null,
    });
    handlers.asset_member_apply_review = ({ ticket }) => ({ plan: plan({ ticket, scope: "whole-group", shared_with: [L8] }), pending: 1, summary: { project_id: 7, dirty: true } });
    mount();
    await pickAndChoose();
    await screen.findByTestId("archive-member-scope");
    expect(screen.getByTestId("archive-member-scope").getAttribute("data-scope")).toBe("this-member");
    expect(screen.getByTestId("archive-member-dialog").getAttribute("data-ticket")).toBe("ns/1");
    expect((screen.getByTestId("archive-member-scope-this") as HTMLInputElement).checked).toBe(true);
    fireEvent.click(screen.getByTestId("archive-member-scope-group"));
    await waitFor(() => expect(screen.getByTestId("archive-member-dialog").getAttribute("data-ticket")).toBe("ns/2"));
    const previews = calls.filter((c) => c.cmd === "asset_member_preview");
    expect(previews.map((c) => c.args.wholeGroup)).toEqual([false, true]);
    expect(screen.getByTestId("archive-member-scope").getAttribute("data-scope")).toBe("whole-group");
    fireEvent.click(screen.getByTestId("archive-member-apply"));
    await screen.findByTestId("archive-member-result");
    expect(calls.find((c) => c.cmd === "asset_member_apply_review")!.args).toEqual({ projectId: 7, ticket: "ns/2" });
  });

  it("shows a server refusal of the preview and never enables Apply for it", async () => {
    handlers.asset_member_preview = () => ({ plan: null, refusal_kind: "texture", refusal: "not a texture member" });
    mount(7);
    await pickAndChoose();
    await screen.findByText(/not a texture member/);
    expect(screen.getByText(/Refused \(texture\)/)).toBeTruthy();
    expect((screen.getByTestId("archive-member-apply") as HTMLButtonElement).disabled).toBe(true);
  });
});
