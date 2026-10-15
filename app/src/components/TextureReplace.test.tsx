// @vitest-environment jsdom
//
// C1 (overnight 2026-09-20) — the loose-file texture replace flow (`useTextureReplace`, the
// League logo / presentation imagery / Uniforms / File Browser "Replace…" route) owns its async
// steps the way the archive-member dialog does since OVERNIGHT-DEV-R1 F1: the picker and the write
// are bound to the install root they started in and to a pick generation. A picker returning after
// the root changed opens nothing; the write always targets the root the pick was made for (never the
// current render's); a write completing after a root change or unmount shows no result and fires no
// callback; "Write texture" dispatches once while one runs; Cancel is inert during a write; the
// writer's notes (a package rebuilt through the generic builder) are shown with the result. Exact
// dispatched arguments are asserted. Component evidence only.
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ImportTextureSummaryDto } from "../bindings/ImportTextureSummaryDto";

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
const pickers: Array<Deferred<string | null>> = [];
vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: () => {
    const d = deferred<string | null>();
    pickers.push(d);
    return d.promise;
  },
  save: () => Promise.resolve(null),
}));

import { useReviewedBytesWrite, useTextureReplace } from "./TextureReplace";
import type { TextureReplacePlanDto } from "../bindings/TextureReplacePlanDto";

const REL = "fe/ion/artassets/leaguelogos/l0.big";
const PREVIEW = { container: "portrait", width: 256, height: 256, data_url: "data:image/png;base64,", thumb_width: 64, thumb_height: 64 };

/** A plan as the backend answers it since the 24 Sep 2026 review: a server-held review id plus the
 *  decoded source dimensions, beside the policy. */
function planOf(over: Partial<TextureReplacePlanDto> = {}): TextureReplacePlanDto {
  return { review_id: BigInt(1), container: "portrait", checked: false, allowed: true, refusal: null, notes: [], source_width: 256, source_height: 256, ...over };
}

function summary(over: Partial<ImportTextureSummaryDto> = {}): ImportTextureSummaryDto {
  return {
    target_path: "E:/x/legacy/fe/ion/artassets/leaguelogos/l0.big",
    container: "portrait",
    width: 256,
    height: 256,
    bytes_written: 8256,
    backup_path: "E:/x/legacy/fe/ion/artassets/leaguelogos/l0.big.bak",
    notes: [],
    ...over,
  };
}

const onWritten = vi.fn();
function Host({ installDir }: { installDir: string | null }) {
  const r = useTextureReplace(installDir, onWritten);
  return (
    <div>
      <button data-testid="pick" onClick={() => void r.pick(REL, "NHL logo")}>
        pick
      </button>
      <span data-testid="pending-rel">{r.pendingRel ?? ""}</span>
      {r.dialog}
      {r.result}
    </div>
  );
}

let qc: QueryClient;
function mount(installDir: string | null = "E:/x/legacy") {
  qc = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } });
  const view = render(
    <QueryClientProvider client={qc}>
      <Host installDir={installDir} />
    </QueryClientProvider>,
  );
  const rerender = (root: string | null) =>
    view.rerender(
      <QueryClientProvider client={qc}>
        <Host installDir={root} />
      </QueryClientProvider>,
    );
  return { ...view, rerender };
}

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
  pickers.length = 0;
  onWritten.mockReset();
  for (const k of Object.keys(handlers)) delete handlers[k];
  handlers.texture_preview = () => PREVIEW;
  handlers.source_image_preview = () => ({ ...PREVIEW, container: null });
  handlers.texture_replace_plan = () => planOf();
});
afterEach(() => cleanup());

describe("useTextureReplace ownership", () => {
  it("previews the current texture in the pick's root and writes with the exact arguments; the result and callback land", async () => {
    handlers.import_texture = () => summary();
    mount();
    await pickAndChoose();
    await screen.findByTestId("texture-replace-dialog");
    const preview = calls.find((c) => c.cmd === "texture_preview");
    expect(preview?.args).toEqual({ installDir: "E:/x/legacy", relPath: REL, maxEdge: 0, textureIndex: undefined });
    await waitFor(() => expect((screen.getByTestId("texture-replace-write") as HTMLButtonElement).disabled).toBe(false));
    fireEvent.click(screen.getByTestId("texture-replace-write"));
    await screen.findByText(/Wrote/);
    const write = calls.filter((c) => c.cmd === "import_texture");
    expect(write).toHaveLength(1);
    expect(write[0].args).toEqual({ installDir: "E:/x/legacy", relPath: REL, sourceImagePath: "C:/tmp/new.png", writeBackup: true, textureIndex: undefined, reviewId: BigInt(1) });
    expect(onWritten).toHaveBeenCalledTimes(1);
    expect(screen.queryByTestId("texture-replace-dialog")).toBeNull();
    expect(screen.queryByTestId("texture-replace-note")).toBeNull();
  });

  it("refreshes cached model and header previews after replacing a texture", async () => {
    handlers.import_texture = () => summary();
    mount("E:/x/legacy");
    for (const key of ["headPreviewPaths", "teamKitPreview", "teamLogo", "players"]) qc.setQueryData([key, "fixture"], "old");
    await pickAndChoose();
    await waitFor(() => expect((screen.getByTestId("texture-replace-write") as HTMLButtonElement).disabled).toBe(false));
    fireEvent.click(screen.getByTestId("texture-replace-write"));
    await screen.findByText(/Wrote/);
    for (const key of ["headPreviewPaths", "teamKitPreview", "teamLogo"]) expect(qc.getQueryState([key, "fixture"])?.isInvalidated).toBe(true);
    expect(qc.getQueryState(["players", "fixture"])?.isInvalidated).toBe(false);
  });

  it("opens nothing when the picker returns after the install root changed, and writes into the pick's root, not the current one", async () => {
    handlers.import_texture = () => summary();
    const v = mount("E:/x/legacy");
    fireEvent.click(screen.getByTestId("pick"));
    await waitFor(() => expect(pickers.length).toBe(1));
    v.rerender("E:/x/nhl15");
    await act(async () => {
      pickers[0].resolve("C:/tmp/late.png");
      await pickers[0].promise;
    });
    expect(screen.queryByTestId("texture-replace-dialog")).toBeNull();
    expect(calls.filter((c) => c.cmd === "texture_preview")).toHaveLength(0);
    // A pick made under the new root, then the root changes while the dialog is open: the dialog
    // closes (nothing held for the old root is shown) and no write is dispatched.
    await pickAndChoose("C:/tmp/second.png");
    await screen.findByTestId("texture-replace-dialog");
    expect(calls.find((c) => c.cmd === "texture_preview")?.args.installDir).toBe("E:/x/nhl15");
    v.rerender("E:/x/legacy");
    await waitFor(() => expect(screen.queryByTestId("texture-replace-dialog")).toBeNull());
    expect(calls.filter((c) => c.cmd === "import_texture")).toHaveLength(0);
  });

  it("neither shows a result nor fires the callback when a write completes after the root changed or the hook unmounted", async () => {
    const w1 = deferred<ImportTextureSummaryDto>();
    handlers.import_texture = () => w1.promise;
    const v = mount("E:/x/legacy");
    await pickAndChoose();
    await waitFor(() => expect((screen.getByTestId("texture-replace-write") as HTMLButtonElement).disabled).toBe(false));
    fireEvent.click(screen.getByTestId("texture-replace-write"));
    expect(calls.filter((c) => c.cmd === "import_texture")[0].args.installDir).toBe("E:/x/legacy");
    v.rerender("E:/x/nhl15");
    await act(async () => {
      w1.resolve(summary());
      await w1.promise;
    });
    expect(screen.queryByText(/Wrote/)).toBeNull();
    expect(onWritten).not.toHaveBeenCalled();

    // Unmount during a write.
    const w2 = deferred<ImportTextureSummaryDto>();
    handlers.import_texture = () => w2.promise;
    await pickAndChoose();
    await waitFor(() => expect((screen.getByTestId("texture-replace-write") as HTMLButtonElement).disabled).toBe(false));
    fireEvent.click(screen.getByTestId("texture-replace-write"));
    v.unmount();
    await act(async () => {
      w2.resolve(summary());
      await w2.promise;
    });
    expect(onWritten).not.toHaveBeenCalled();
  });

  it("dispatches one write while one is in flight, keeps Cancel inert, refuses a new pick, and lands the completion", async () => {
    const w = deferred<ImportTextureSummaryDto>();
    handlers.import_texture = () => w.promise;
    mount();
    await pickAndChoose();
    await waitFor(() => expect((screen.getByTestId("texture-replace-write") as HTMLButtonElement).disabled).toBe(false));
    const btn = screen.getByTestId("texture-replace-write");
    fireEvent.click(btn);
    fireEvent.click(btn);
    fireEvent.click(btn);
    await screen.findByTestId("texture-replace-writing");
    expect(calls.filter((c) => c.cmd === "import_texture")).toHaveLength(1);
    expect((screen.getByTestId("texture-replace-cancel") as HTMLButtonElement).disabled).toBe(true);
    fireEvent.click(screen.getByTestId("texture-replace-cancel"));
    fireEvent.click(screen.getByTestId("pick"));
    expect(pickers).toHaveLength(1); // no second picker while writing
    expect(screen.getByTestId("texture-replace-dialog")).toBeTruthy();
    await act(async () => {
      w.resolve(summary({ notes: ["package shape not recognised by the span-preserving re-plan (x); the package was rebuilt through the generic builder"] }));
      await w.promise;
    });
    await screen.findByText(/Wrote/);
    expect(screen.getByTestId("texture-replace-note").textContent).toMatch(/rebuilt through the generic builder/);
    expect(onWritten).toHaveBeenCalledTimes(1);
    expect(screen.getByTestId("pending-rel").textContent).toBe("");
  });

  it("shows a write failure in the dialog and keeps the pick so the user can retry or cancel", async () => {
    handlers.import_texture = () => Promise.reject(new Error("could not write: permission denied"));
    mount();
    await pickAndChoose();
    await waitFor(() => expect((screen.getByTestId("texture-replace-write") as HTMLButtonElement).disabled).toBe(false));
    fireEvent.click(screen.getByTestId("texture-replace-write"));
    await screen.findByText(/permission denied/);
    expect(screen.getByTestId("texture-replace-dialog")).toBeTruthy();
    expect(onWritten).not.toHaveBeenCalled();
    fireEvent.click(screen.getByTestId("texture-replace-cancel"));
    expect(screen.queryByTestId("texture-replace-dialog")).toBeNull();
  });

  it("blocks the write on a size mismatch for a same-size container and never dispatches it", async () => {
    // No same-size-only loose container is left after P2-B (portrait / png / texlib / gnf all
    // resize under their rules); an unknown container stays blocked.
    handlers.source_image_preview = () => ({ ...PREVIEW, container: null, width: 128, height: 128 });
    handlers.texture_preview = () => ({ ...PREVIEW, container: "mystery" });
    handlers.import_texture = () => summary();
    mount();
    await pickAndChoose();
    await screen.findByText(/Size mismatch/);
    expect((screen.getByTestId("texture-replace-write") as HTMLButtonElement).disabled).toBe(true);
    fireEvent.click(screen.getByTestId("texture-replace-write"));
    expect(calls.filter((c) => c.cmd === "import_texture")).toHaveLength(0);
  });

  it("asks the writer's plan with the source dimensions and shows a refusal before the write, never dispatching it", async () => {
    // Coordinator review of receipts 10–17, F1: a changed size on a mipped Xbox 360 raster is
    // refused, and the dialog says so before Apply instead of letting the write fail.
    handlers.source_image_preview = () => ({ ...PREVIEW, container: null, width: 128, height: 128 });
    handlers.texture_preview = () => ({ ...PREVIEW, container: "texlib" });
    handlers.texture_replace_plan = () =>
      planOf({
        container: "texlib",
        checked: true,
        allowed: false,
        refusal: "Xbox 360 changed-size replacement refused: this raster declares 10 mip levels … Nothing was written.",
        source_width: 128,
        source_height: 128,
      });
    handlers.import_texture = () => summary();
    mount();
    await pickAndChoose();
    await screen.findByTestId("texture-replace-plan-refusal");
    const planCall = calls.find((c) => c.cmd === "texture_replace_plan");
    expect(planCall?.args).toEqual({ installDir: "E:/x/legacy", relPath: REL, sourceImagePath: "C:/tmp/new.png", textureIndex: undefined });
    expect(screen.getByTestId("texture-replace-plan-refusal").textContent).toMatch(/declares 10 mip levels/);
    expect((screen.getByTestId("texture-replace-write") as HTMLButtonElement).disabled).toBe(true);
    fireEvent.click(screen.getByTestId("texture-replace-write"));
    expect(calls.filter((c) => c.cmd === "import_texture")).toHaveLength(0);
  });

  it("shows the plan's partial-replacement disclosure before the write and the same note with the result", async () => {
    const note = "Partial replacement (Xbox 360 mip chain): the base and 2 smaller levels above 64 texels are rewritten …; the 7 packed-tail levels keep the shipped bytes …";
    handlers.texture_preview = () => ({ ...PREVIEW, container: "texlib" });
    handlers.texture_replace_plan = () => planOf({ container: "texlib", checked: true, notes: [note] });
    handlers.import_texture = () => summary({ container: "texlib", notes: [note] });
    mount();
    await pickAndChoose();
    await screen.findByTestId("texture-replace-plan-note");
    expect(screen.getByTestId("texture-replace-plan-note").textContent).toMatch(/Partial replacement/);
    await waitFor(() => expect((screen.getByTestId("texture-replace-write") as HTMLButtonElement).disabled).toBe(false));
    fireEvent.click(screen.getByTestId("texture-replace-write"));
    await screen.findByText(/Wrote/);
    expect(screen.getByTestId("texture-replace-note").textContent).toMatch(/Partial replacement/);
  });

  // R4 review F1 (23 Sep 2026): a partial replacement needs its disclosure shown first, so the
  // write gate waits for a current successful plan. Pending, failed, retried, switched and stale
  // plans are each exercised; no import is dispatched before the current disclosure is shown.
  it("keeps Write disabled while the plan is pending and never dispatches until it answers", async () => {
    const held = deferred<any>();
    handlers.texture_preview = () => ({ ...PREVIEW, container: "texlib" });
    handlers.texture_replace_plan = () => held.promise;
    handlers.import_texture = () => summary({ container: "texlib" });
    mount();
    await pickAndChoose();
    await screen.findByTestId("texture-replace-plan-loading");
    const btn = screen.getByTestId("texture-replace-write") as HTMLButtonElement;
    expect(btn.disabled).toBe(true);
    fireEvent.click(btn);
    expect(calls.filter((c) => c.cmd === "import_texture")).toHaveLength(0);
    await act(async () => {
      held.resolve(planOf({ container: "texlib", checked: true, notes: ["Partial replacement (Xbox 360 mip chain): …"] }));
      await held.promise;
    });
    await screen.findByTestId("texture-replace-plan-note");
    await waitFor(() => expect((screen.getByTestId("texture-replace-write") as HTMLButtonElement).disabled).toBe(false));
    fireEvent.click(screen.getByTestId("texture-replace-write"));
    await screen.findByText(/Wrote/);
    expect(calls.filter((c) => c.cmd === "import_texture")).toHaveLength(1);
  });

  it("keeps Write disabled when the plan fails, offers Retry, and enables only after a successful answer", async () => {
    let attempts = 0;
    handlers.texture_preview = () => ({ ...PREVIEW, container: "texlib" });
    handlers.texture_replace_plan = () => {
      attempts += 1;
      return attempts === 1 ? Promise.reject(new Error("could not read")) : planOf({ container: "texlib", checked: true, notes: ["Partial replacement …"] });
    };
    handlers.import_texture = () => summary({ container: "texlib" });
    mount();
    await pickAndChoose();
    await screen.findByTestId("texture-replace-plan-unavailable");
    expect((screen.getByTestId("texture-replace-write") as HTMLButtonElement).disabled).toBe(true);
    fireEvent.click(screen.getByTestId("texture-replace-write"));
    expect(calls.filter((c) => c.cmd === "import_texture")).toHaveLength(0);
    // Cancel stays available on a failed check.
    expect((screen.getByTestId("texture-replace-cancel") as HTMLButtonElement).disabled).toBe(false);
    fireEvent.click(screen.getByTestId("texture-replace-plan-retry"));
    await screen.findByTestId("texture-replace-plan-note");
    expect(attempts).toBe(2);
    await waitFor(() => expect((screen.getByTestId("texture-replace-write") as HTMLButtonElement).disabled).toBe(false));
    fireEvent.click(screen.getByTestId("texture-replace-write"));
    await screen.findByText(/Wrote/);
    expect(calls.filter((c) => c.cmd === "import_texture")).toHaveLength(1);
  });

  it("re-plans for a switched source and shows that source's refusal, never the previous source's policy", async () => {
    handlers.texture_preview = () => ({ ...PREVIEW, container: "texlib" });
    const sizes: Record<string, number> = { "C:/tmp/same.png": 256, "C:/tmp/half.png": 128 };
    handlers.source_image_preview = (a: any) => ({ ...PREVIEW, container: null, width: sizes[a.sourceImagePath], height: sizes[a.sourceImagePath] });
    handlers.texture_replace_plan = (a: any) =>
      a.sourceImagePath === "C:/tmp/same.png"
        ? planOf({ container: "texlib", checked: true, notes: ["Partial replacement …"] })
        : planOf({ container: "texlib", checked: true, allowed: false, refusal: "Xbox 360 changed-size replacement refused: … a 128×128 chain …", source_width: 128, source_height: 128 });
    handlers.import_texture = () => summary({ container: "texlib" });
    mount();
    await pickAndChoose("C:/tmp/same.png");
    await screen.findByTestId("texture-replace-plan-note");
    await waitFor(() => expect((screen.getByTestId("texture-replace-write") as HTMLButtonElement).disabled).toBe(false));
    fireEvent.click(screen.getByTestId("texture-replace-cancel"));
    await pickAndChoose("C:/tmp/half.png");
    await screen.findByTestId("texture-replace-plan-refusal");
    expect(screen.getByTestId("texture-replace-plan-refusal").textContent).toContain("128×128");
    expect(screen.queryByTestId("texture-replace-plan-note")).toBeNull();
    expect((screen.getByTestId("texture-replace-write") as HTMLButtonElement).disabled).toBe(true);
    fireEvent.click(screen.getByTestId("texture-replace-write"));
    expect(calls.filter((c) => c.cmd === "import_texture")).toHaveLength(0);
    const plans = calls.filter((c) => c.cmd === "texture_replace_plan");
    expect(plans.map((p) => p.args.sourceImagePath)).toEqual(["C:/tmp/same.png", "C:/tmp/half.png"]);
  });

  it("re-reads the target and re-plans on every open, so a file replaced at the same path never inherits a stale plan", async () => {
    let generation = 0;
    handlers.texture_preview = () => ({ ...PREVIEW, container: "texlib" });
    handlers.texture_replace_plan = () => {
      generation += 1;
      return generation === 1
        ? planOf({ container: "texlib", checked: true, notes: ["Partial replacement …"] })
        : planOf({ container: "texlib", checked: true, allowed: false, refusal: "Xbox 360 changed-size replacement refused: this raster declares 10 mip levels …" });
    };
    handlers.import_texture = () => summary({ container: "texlib" });
    mount();
    await pickAndChoose("C:/tmp/same.png");
    await screen.findByTestId("texture-replace-plan-note");
    fireEvent.click(screen.getByTestId("texture-replace-cancel"));
    // Same target, same source path, same dimensions: the file at that path changed meanwhile.
    await pickAndChoose("C:/tmp/same.png");
    await screen.findByTestId("texture-replace-plan-refusal");
    expect((screen.getByTestId("texture-replace-write") as HTMLButtonElement).disabled).toBe(true);
    fireEvent.click(screen.getByTestId("texture-replace-write"));
    expect(calls.filter((c) => c.cmd === "import_texture")).toHaveLength(0);
    expect(calls.filter((c) => c.cmd === "texture_replace_plan")).toHaveLength(2);
    expect(calls.filter((c) => c.cmd === "texture_preview")).toHaveLength(2);
  });

  it("offers a texlib / GNF resize with the rule's qualification and dispatches the write", async () => {
    for (const [container, text] of [
      ["texlib", /single-level raster changes size/],
      ["gnf", /registry's size directory still decides/],
    ] as const) {
      cleanup();
      calls.length = 0;
      handlers.source_image_preview = () => ({ ...PREVIEW, container: null, width: 128, height: 128 });
      handlers.texture_preview = () => ({ ...PREVIEW, container });
      handlers.import_texture = () => summary();
      mount();
      await pickAndChoose();
      await screen.findByTestId("texture-replace-resize");
      expect(screen.getByTestId("texture-replace-resize").textContent).toMatch(text);
      await waitFor(() => expect((screen.getByTestId("texture-replace-write") as HTMLButtonElement).disabled).toBe(false));
      fireEvent.click(screen.getByTestId("texture-replace-write"));
      await waitFor(() => expect(calls.filter((c) => c.cmd === "import_texture")).toHaveLength(1));
    }
  });
  // 24 Sep 2026 coordinator review F1: the write names the displayed review. A refused write (the
  // target or source changed after the plan was shown) shows the refusal, re-plans so a *fresh*
  // review is displayed, and dispatches nothing on its own; the next Write names the new review.
  it("names the displayed review in the write, and after a drift refusal re-plans and displays the fresh review without writing on its own", async () => {
    let generation = 0;
    handlers.texture_preview = () => ({ ...PREVIEW, container: "texlib" });
    handlers.texture_replace_plan = () => {
      generation += 1;
      return generation === 1
        ? planOf({ review_id: BigInt(11), container: "texlib", checked: true, notes: [] })
        : planOf({ review_id: BigInt(12), container: "texlib", checked: true, notes: ["Partial replacement (Xbox 360 mip chain): …"] });
    };
    let writes = 0;
    handlers.import_texture = (a: any) => {
      writes += 1;
      if (writes === 1) {
        return Promise.reject(
          new Error("The reviewed write was not performed: the texture file changed since the plan was shown (the reviewed bytes are not the bytes on disk). Nothing was written and no backup was made; review the replacement again before writing."),
        );
      }
      expect(a.reviewId).toBe(BigInt(12));
      return summary({ container: "texlib", notes: ["Partial replacement (Xbox 360 mip chain): …"] });
    };
    mount();
    await pickAndChoose();
    await waitFor(() => expect((screen.getByTestId("texture-replace-write") as HTMLButtonElement).disabled).toBe(false));
    expect(screen.queryByTestId("texture-replace-plan-note")).toBeNull();
    fireEvent.click(screen.getByTestId("texture-replace-write"));
    // The refusal is shown; the plan is re-read; the fresh review's disclosure is displayed.
    await screen.findByText(/changed since the plan was shown/);
    await screen.findByTestId("texture-replace-plan-note");
    expect(calls.filter((c) => c.cmd === "texture_replace_plan")).toHaveLength(2);
    expect(calls.filter((c) => c.cmd === "import_texture")).toHaveLength(1);
    expect(calls.filter((c) => c.cmd === "import_texture")[0].args.reviewId).toBe(BigInt(11));
    // Nothing wrote itself: the operator decides on the fresh review.
    await waitFor(() => expect((screen.getByTestId("texture-replace-write") as HTMLButtonElement).disabled).toBe(false));
    fireEvent.click(screen.getByTestId("texture-replace-write"));
    await screen.findByText(/Wrote/);
    expect(calls.filter((c) => c.cmd === "import_texture")).toHaveLength(2);
    expect(calls.filter((c) => c.cmd === "import_texture")[1].args.reviewId).toBe(BigInt(12));
    expect(screen.getByTestId("texture-replace-note").textContent).toMatch(/Partial replacement/);
  });
});

// The bytes route (the nameplate editor's Save) as a reviewed two-step write.
function BytesHost({ installDir, relPath, textureIndex }: { installDir: string; relPath: string; textureIndex?: number }) {
  const r = useReviewedBytesWrite(installDir, relPath, textureIndex);
  return (
    <div>
      <button data-testid="stage" onClick={() => void r.stage("UE5H")}>stage</button>
      <button data-testid="write" onClick={() => void r.write()} disabled={!r.canWrite}>write</button>
      <button data-testid="reset" onClick={r.reset}>reset</button>
      <span data-testid="state">
        {r.planning ? "planning" : r.writing ? "writing" : r.result ? "written" : r.plan ? (r.refused ? "refused" : "planned") : "idle"}
      </span>
      <span data-testid="plan-error">{r.planError ?? ""}</span>
      <span data-testid="write-error">{r.writeError ?? ""}</span>
      <span data-testid="notes">{r.plan?.notes.join("|") ?? ""}</span>
    </div>
  );
}
function mountBytes(props: { installDir: string; relPath: string; textureIndex?: number }) {
  qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  const view = render(
    <QueryClientProvider client={qc}>
      <BytesHost {...props} />
    </QueryClientProvider>,
  );
  return {
    ...view,
    rerender: (p: { installDir: string; relPath: string; textureIndex?: number }) =>
      view.rerender(
        <QueryClientProvider client={qc}>
          <BytesHost {...p} />
        </QueryClientProvider>,
      ),
  };
}

describe("useReviewedBytesWrite (nameplate editor Save)", () => {
  const T = { installDir: "E:/x/legacy", relPath: "rendering/jersey/names.rx2", textureIndex: 0 };

  it("stages the baked PNG as a review, shows the disclosure, writes exactly once under that review id, and a spent review cannot write again", async () => {
    handlers.texture_replace_plan_bytes = () => planOf({ review_id: BigInt(5), container: "texlib", checked: true, notes: ["Partial replacement (Xbox 360 mip chain): …"] });
    handlers.import_texture_bytes = () => summary({ container: "texlib", notes: ["Partial replacement (Xbox 360 mip chain): …"] });
    mountBytes(T);
    expect((screen.getByTestId("write") as HTMLButtonElement).disabled).toBe(true);
    fireEvent.click(screen.getByTestId("stage"));
    await waitFor(() => expect(screen.getByTestId("state").textContent).toBe("planned"));
    expect(calls.find((c) => c.cmd === "texture_replace_plan_bytes")?.args).toEqual({ ...T, pngBase64: "UE5H" });
    expect(screen.getByTestId("notes").textContent).toMatch(/Partial replacement/);
    fireEvent.click(screen.getByTestId("write"));
    fireEvent.click(screen.getByTestId("write"));
    await waitFor(() => expect(screen.getByTestId("state").textContent).toBe("written"));
    const writes = calls.filter((c) => c.cmd === "import_texture_bytes");
    expect(writes).toHaveLength(1);
    expect(writes[0].args).toEqual({ ...T, writeBackup: true, reviewId: BigInt(5) });
    // No plan is held any more: Write is disabled until a new stage.
    expect((screen.getByTestId("write") as HTMLButtonElement).disabled).toBe(true);
  });

  it("never writes a refused review, and a failed or drifted write drops the review so a fresh plan must be shown", async () => {
    let n = 0;
    handlers.texture_replace_plan_bytes = () => {
      n += 1;
      return n === 1
        ? planOf({ review_id: BigInt(1), container: "texlib", checked: true, allowed: false, refusal: "Xbox 360 changed-size replacement refused: this raster declares 10 mip levels …" })
        : planOf({ review_id: BigInt(2), container: "texlib", checked: true, notes: ["Partial replacement …"] });
    };
    handlers.import_texture_bytes = () => Promise.reject(new Error("The reviewed write was not performed: the texture file changed since the plan was shown …"));
    mountBytes(T);
    fireEvent.click(screen.getByTestId("stage"));
    await waitFor(() => expect(screen.getByTestId("state").textContent).toBe("refused"));
    expect((screen.getByTestId("write") as HTMLButtonElement).disabled).toBe(true);
    fireEvent.click(screen.getByTestId("write"));
    expect(calls.filter((c) => c.cmd === "import_texture_bytes")).toHaveLength(0);
    // A second bake plans afresh; the write then drifts.
    fireEvent.click(screen.getByTestId("stage"));
    await waitFor(() => expect(screen.getByTestId("state").textContent).toBe("planned"));
    fireEvent.click(screen.getByTestId("write"));
    await waitFor(() => expect(screen.getByTestId("write-error").textContent).toMatch(/changed since the plan was shown/));
    expect(screen.getByTestId("state").textContent).toBe("idle");
    expect((screen.getByTestId("write") as HTMLButtonElement).disabled).toBe(true);
    expect(calls.filter((c) => c.cmd === "import_texture_bytes")).toHaveLength(1);
    expect(calls.filter((c) => c.cmd === "texture_replace_plan_bytes")).toHaveLength(2);
  });

  it("reports a failed pre-check without a review, and drops a review when the target changes or a newer stage supersedes it", async () => {
    const held = deferred<TextureReplacePlanDto>();
    let n = 0;
    handlers.texture_replace_plan_bytes = () => {
      n += 1;
      return n === 1 ? Promise.reject(new Error("could not read")) : n === 2 ? held.promise : planOf({ review_id: BigInt(9), container: "texlib", checked: true });
    };
    handlers.import_texture_bytes = () => summary({ container: "texlib" });
    const v = mountBytes(T);
    fireEvent.click(screen.getByTestId("stage"));
    await waitFor(() => expect(screen.getByTestId("plan-error").textContent).toMatch(/could not read/));
    expect(screen.getByTestId("state").textContent).toBe("idle");
    // A stage superseded by a newer stage: the older answer is discarded when it arrives.
    fireEvent.click(screen.getByTestId("stage"));
    await waitFor(() => expect(screen.getByTestId("state").textContent).toBe("planning"));
    fireEvent.click(screen.getByTestId("stage"));
    await waitFor(() => expect(screen.getByTestId("state").textContent).toBe("planned"));
    await act(async () => {
      held.resolve(planOf({ review_id: BigInt(8), container: "texlib", checked: true, notes: ["stale"] }));
      await held.promise;
    });
    expect(screen.getByTestId("notes").textContent).toBe("");
    fireEvent.click(screen.getByTestId("write"));
    await waitFor(() => expect(screen.getByTestId("state").textContent).toBe("written"));
    expect(calls.filter((c) => c.cmd === "import_texture_bytes")[0].args.reviewId).toBe(BigInt(9));
    // A target change discards the held state.
    fireEvent.click(screen.getByTestId("stage"));
    await waitFor(() => expect(screen.getByTestId("state").textContent).toBe("planned"));
    v.rerender({ ...T, relPath: "rendering/jersey/other.rx2" });
    await waitFor(() => expect(screen.getByTestId("state").textContent).toBe("idle"));
    expect((screen.getByTestId("write") as HTMLButtonElement).disabled).toBe(true);
  });
});
