// @vitest-environment jsdom
//
// OVERNIGHT-DEV-R2 F2 — the disc-image open panel: inspection results are bound to the image
// they were requested for (a late inspection never describes the field's new image), the open
// refuses an image whose inspection found no database, and the opened project is handed to the
// shell only while the panel still owns the request. Component evidence only (jsdom, mocked
// commands and dialog): not a desktop run.
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { IsoInspectDto } from "../bindings/IsoInspectDto";
import type { IsoStagedDto } from "../bindings/IsoStagedDto";
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
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: () => Promise.resolve(null) }));

import { IsoOpenPanel } from "./IsoOpenPanel";

function inspection(image: string, withDb: boolean): IsoInspectDto {
  return {
    image,
    layout: "xgd2",
    members: 1200,
    nhlng: withDb
      ? { member_path: "cacheboot.big", member_sector: 40, member_size: 1000, entry_path: "db/nhlng.db", entry_index: 3, entry_offset: BigInt(64), stored_size: 500, describe: `${image}: cacheboot.big › db/nhlng.db` }
      : null,
    nhlng_sha256: withDb ? "abcdef123456" : null,
    nhlng_len: withDb ? BigInt(500) : null,
    nhlng_tables: withDb ? 134 : null,
    dictionary: null,
    dictionary_sha256: null,
    dictionary_len: null,
    dictionary_rows: null,
    probed: [{ path: "cacheboot.big", size: 1000, verdict: "bigf" }],
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
  dictionary_path: null,
  files: [],
  reused: false,
  staged_at: "2026-09-18T05:00:00Z",
  project_id: 4,
};
const summary = { project_id: 4, dirty: false, workspace_revision: "rev-a" } as ProjectSummaryDto;

function mount(onOpened = vi.fn()) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  const el = () => (
    <QueryClientProvider client={qc}>
      <IsoOpenPanel gameId="nhl14" platform="xbox" defaultStagingRoot="E:/staging/nhl14" onOpened={onOpened} />
    </QueryClientProvider>
  );
  const r = render(el());
  return { ...r, onOpened };
}

const imageInput = () => screen.getByLabelText("disc image path") as HTMLInputElement;
const openButton = () => screen.getByText(/Stage and open/) as HTMLButtonElement;

beforeEach(() => {
  calls.length = 0;
  for (const k of Object.keys(handlers)) delete handlers[k];
});
afterEach(() => cleanup());

describe("IsoOpenPanel", () => {
  it("passes a linked asset installation to the shell with the staged database", async () => {
    const install = { install_dir: "E:/games/copied NHL", platform: "xbox" };
    handlers.detect_install = () => install;
    handlers.iso_open = () => [staged, summary];
    const { onOpened } = mount();
    fireEvent.change(imageInput(), { target: { value: "X:/NHL 14.iso" } });
    fireEvent.change(screen.getByLabelText("ISO game files folder"), { target: { value: install.install_dir } });
    fireEvent.click(openButton());
    await waitFor(() => expect(onOpened).toHaveBeenCalledWith(staged, summary, install));
  });

  it("does not open with an unrecognized asset folder", async () => {
    handlers.detect_install = () => ({ install_dir: null });
    const { onOpened } = mount();
    fireEvent.change(imageInput(), { target: { value: "X:/NHL 14.iso" } });
    fireEvent.change(screen.getByLabelText("ISO game files folder"), { target: { value: "E:/wrong" } });
    fireEvent.click(openButton());
    await waitFor(() => expect(screen.getByText(/game files folder was not recognized/)).toBeTruthy());
    expect(onOpened).not.toHaveBeenCalled();
    expect(calls.some(c => c.cmd === "iso_open")).toBe(false);
  });

  it("blocks opening a cancelled disc copy and offers cancellation while copying", async () => {
    const running = { job: 5, root: "E:/games/new", state: "running", current: "Copying disc" };
    handlers.disc_extract_start = () => running;
    handlers.extract_status = () => running;
    handlers.extract_cancel = () => ({ ...running, state: "cancelled", error: "Incomplete copy; choose a new folder" });
    mount();
    fireEvent.change(imageInput(), { target: { value: "X:/NHL 14.iso" } });
    fireEvent.change(screen.getByLabelText("ISO game files folder"), { target: { value: running.root } });
    fireEvent.click(screen.getByText("Extract full disc"));
    await waitFor(() => expect(screen.getByText("Cancel disc extraction")).toBeTruthy());
    expect(openButton().disabled).toBe(true);
    fireEvent.click(screen.getByText("Cancel disc extraction"));
    await waitFor(() => expect(screen.getByText(/Incomplete copy; choose a new folder/)).toBeTruthy());
    expect(openButton().disabled).toBe(true);
    expect(calls.some(c => c.cmd === "iso_open")).toBe(false);
  });

  it("offers archive extraction after the whole disc finishes copying", async () => {
    handlers.disc_extract_start = () => ({ job: 6, root: "E:/games/new", state: "done", current: "Disc copied" });
    handlers.extract_running_for = () => null;
    handlers.extract_preflight = () => ({ state: "needed", reason: "Packed assets remain", archives: [], collisions: [], members_total: 2, members_present: 0, members_to_write: 2, bytes_to_write: 10, free_bytes: 1000 });
    mount();
    fireEvent.change(imageInput(), { target: { value: "X:/NHL 14.iso" } });
    fireEvent.change(screen.getByLabelText("ISO game files folder"), { target: { value: "E:/games/new" } });
    fireEvent.click(screen.getByText("Extract full disc"));
    await screen.findByTestId("extraction-preflight");
    expect((screen.getByLabelText("Extraction destination") as HTMLInputElement).value).toBe("E:/games/new");
    expect(calls.find(c => c.cmd === "extract_preflight")?.args.root).toBe("E:/games/new");
  });

  it("inspects the image in the field and shows where the database lives", async () => {
    handlers.iso_inspect = ({ image }: { image: string }) => inspection(image, true);
    mount();
    expect(openButton().disabled).toBe(true);
    fireEvent.change(imageInput(), { target: { value: "X:/NHL 14.iso" } });
    fireEvent.click(screen.getByText("Inspect"));
    await waitFor(() => expect(screen.getByTestId("iso-inspection")).toBeTruthy());
    expect(screen.getByText(/cacheboot.big › db\/nhlng.db/)).toBeTruthy();
    expect(calls.find((c) => c.cmd === "iso_inspect")!.args).toEqual({ image: "X:/NHL 14.iso" });
    expect(openButton().disabled).toBe(false);
  });

  it("drops a late inspection after the image field changed", async () => {
    const pending = deferred<IsoInspectDto>();
    handlers.iso_inspect = () => pending.promise;
    mount();
    fireEvent.change(imageInput(), { target: { value: "X:/NHL 14.iso" } });
    fireEvent.click(screen.getByText("Inspect"));
    await waitFor(() => expect(calls.some((c) => c.cmd === "iso_inspect")).toBe(true));
    fireEvent.change(imageInput(), { target: { value: "X:/NHL 15.iso" } });
    await act(async () => {
      pending.resolve(inspection("X:/NHL 14.iso", true));
      await pending.promise;
    });
    expect(screen.queryByTestId("iso-inspection")).toBeNull();
  });

  it("refuses to open an image whose inspection found no database", async () => {
    handlers.iso_inspect = ({ image }: { image: string }) => inspection(image, false);
    mount();
    fireEvent.change(imageInput(), { target: { value: "X:/not-nhl.iso" } });
    fireEvent.click(screen.getByText("Inspect"));
    await waitFor(() => expect(screen.getByText(/not found in this image/)).toBeTruthy());
    expect(openButton().disabled).toBe(true);
    expect(calls.some((c) => c.cmd === "iso_open")).toBe(false);
  });

  it("stages and opens with the title, platform and staging folder, and hands the project to the shell", async () => {
    handlers.iso_open = () => [staged, summary];
    const { onOpened } = mount();
    fireEvent.change(imageInput(), { target: { value: "X:/NHL 14.iso" } });
    fireEvent.click(openButton());
    await waitFor(() => expect(onOpened).toHaveBeenCalledTimes(1));
    const args = calls.find((c) => c.cmd === "iso_open")!.args;
    expect(args).toEqual({ image: "X:/NHL 14.iso", gameId: "nhl14", platform: "xbox", stagingRoot: "E:/staging/nhl14", metaXml: null });
    expect(onOpened.mock.calls[0][0].project_id).toBe(4);
  });

  it("does not hand a late open to the shell after the image field changed", async () => {
    const pending = deferred<[IsoStagedDto, ProjectSummaryDto]>();
    handlers.iso_open = () => pending.promise;
    const { onOpened } = mount();
    fireEvent.change(imageInput(), { target: { value: "X:/NHL 14.iso" } });
    fireEvent.click(openButton());
    await waitFor(() => expect(calls.some((c) => c.cmd === "iso_open")).toBe(true));
    fireEvent.change(imageInput(), { target: { value: "X:/NHL 15.iso" } });
    await act(async () => {
      pending.resolve([staged, summary]);
      await pending.promise;
    });
    expect(onOpened).not.toHaveBeenCalled();
  });

  it("shows the server's refusal when the open fails", async () => {
    handlers.iso_open = () => Promise.reject(new Error("E:/staging/nhl14 already stages X:/NHL 13.iso; choose another staging folder"));
    mount();
    fireEvent.change(imageInput(), { target: { value: "X:/NHL 14.iso" } });
    fireEvent.click(openButton());
    await waitFor(() => expect(screen.getByText(/already stages/)).toBeTruthy());
  });
});
