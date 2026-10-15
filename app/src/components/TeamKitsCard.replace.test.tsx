// @vitest-environment jsdom
//
// Teams 2.6 (overnight 2026-09-24, package T2) — the kit card offers Replace… on present components:
// a loose file through the caller's reviewed loose route (exact rel path + label), a render-archive
// member (PS3 NHL 12 / 13) through the archive-member route (the picker, then `asset_member_preview`
// with the exact archive path + member path in the card's install root); the NHL 11 / 12 sock pair is
// not offered file-by-file and says why; missing components offer nothing. jsdom only.
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { TeamKitsDto } from "../bindings/TeamKitsDto";
import type { KitComponentDto } from "../bindings/KitComponentDto";

const calls: Array<{ cmd: string; args: any }> = [];
const handlers: Record<string, (args: any) => unknown> = {};
vi.mock("../lib/client", () => ({
  call: (cmd: string, args: any) => {
    calls.push({ cmd, args });
    const h = handlers[cmd];
    if (!h) return new Promise(() => {}); // thumbnails etc. stay pending
    return Promise.resolve().then(() => h(args));
  },
}));
type Deferred<T> = { promise: Promise<T>; resolve: (v: T) => void };
const pickers: Deferred<string | null>[] = [];
vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: () => {
    let resolve!: (v: string | null) => void;
    const promise = new Promise<string | null>((r) => (resolve = r));
    pickers.push({ promise, resolve });
    return promise;
  },
}));
vi.mock("../lib/project", () => ({ useProject: () => ({ update: () => {} }) }));

import { TeamKitsCard } from "./TeamKitsCard";

const roots = { install_dir: "E:/copy", platform: "ps3", rendering_override: "E:/copy/PS3_GAME/USRDIR/rendering", fe_override: null };
function comp(over: Partial<KitComponentDto>): KitComponentDto {
  return { part: "jersey", label: "jersey", file_stem: "x", path: null, rel_path: null, archive_member: null, archive_rel_path: null, member_path: null, status: "present", reason: null, ...over };
}
function payload(sockGrammar: string, components: KitComponentDto[]): TeamKitsDto {
  return {
    team_id: 5,
    team_art_id: 7,
    contract_status: "measured",
    sock_grammar: sockGrammar,
    contract: "contract",
    rendering_root: "E:/copy/PS3_GAME/USRDIR/rendering",
    directories: [],
    kits: [{ record: 10, jersey_index: 0, style: 0, team_art_id: 7, variant: 0, is_default: true, is_light_color: false, has_tie: false, helmet_logo: true, helmet_numbers: false, components, completeness: "complete", refusal: null }],
    spares: [],
    reassignment_supported: false,
    reassignment_reason: "no contract",
    notes: [],
  } as unknown as TeamKitsDto;
}
const onReplaceLoose = vi.fn();
function mount() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <TeamKitsCard projectId={3} teamId={5} rootArgs={roots} installDir="E:/copy" style={0} variant={0} onReplaceLoose={onReplaceLoose} />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  calls.length = 0;
  pickers.length = 0;
  onReplaceLoose.mockReset();
  for (const k of Object.keys(handlers)) delete handlers[k];
});
afterEach(cleanup);

describe("TeamKitsCard replacement affordances", () => {
  it("offers Replace… on a present loose component through the caller's route, and Replace in archive… on an archived one through the member route", async () => {
    handlers.team_kits = () =>
      payload("single_file", [
        comp({ part: "jersey", label: "jersey", rel_path: "rendering/jersey/texlib_0_7_0.rx2", path: "E:/copy/rendering/jersey/texlib_0_7_0.rx2" }),
        comp({ part: "pant", label: "pants", archive_rel_path: "PS3_GAME/USRDIR/cacherender.big", member_path: "rendering/pant/texlib_0_7_0.rpsgl", archive_member: "cacherender.big:rendering/pant/texlib_0_7_0.rpsgl" }),
        comp({ part: "sock", label: "socks", status: "missing", reason: "not shipped" }),
      ]);
    handlers.asset_member_preview = () => ({ plan: null, refusal_kind: "not-found", refusal: "no such member" });
    mount();
    await screen.findByTestId("kit-component-replace-jersey");
    fireEvent.click(screen.getByTestId("kit-component-replace-jersey"));
    expect(onReplaceLoose).toHaveBeenCalledWith("rendering/jersey/texlib_0_7_0.rx2", "jersey");
    // The missing component offers nothing.
    expect(screen.queryByTestId("kit-component-replace-socks")).toBeNull();
    expect(screen.queryByTestId("kit-component-replace-archived-socks")).toBeNull();
    // The archived component: the picker opens, then the member route is asked with the exact addresses.
    fireEvent.click(screen.getByTestId("kit-component-replace-archived-pants"));
    await waitFor(() => expect(pickers).toHaveLength(1));
    pickers[0].resolve("C:/tmp/pants.png");
    await waitFor(() => expect(calls.filter((c) => c.cmd === "asset_member_preview")).toHaveLength(1));
    expect(calls.find((c) => c.cmd === "asset_member_preview")!.args).toEqual({
      projectId: 3,
      installRoot: "E:/copy",
      archiveRelative: "PS3_GAME/USRDIR/cacherender.big",
      memberPath: "rendering/pant/texlib_0_7_0.rpsgl",
      sourceImagePath: "C:/tmp/pants.png",
      wholeGroup: false,
    });
    // The refusal is shown by the member dialog; nothing else was dispatched.
    await screen.findByText(/no such member/);
    expect(calls.filter((c) => c.cmd === "asset_member_apply_review")).toHaveLength(0);
  });

  it("does not offer a file-by-file replace on the NHL 11 / 12 sock pair and says why", async () => {
    handlers.team_kits = () =>
      payload("two_files", [
        comp({ part: "sock", label: "socks (file 0)", rel_path: "rendering/sock/sock_0_0_7_0_cm.rx2", path: "x" }),
        comp({ part: "sock", label: "socks (file 1)", rel_path: "rendering/sock/sock_0_1_7_0_cm.rx2", path: "x" }),
        comp({ part: "pant", label: "pants", rel_path: "rendering/pant/texlib_0_7_0.rx2", path: "x" }),
      ]);
    mount();
    await screen.findByTestId("kit-component-pair-socks-file-0-");
    expect(screen.getByTestId("kit-component-pair-socks-file-0-").textContent).toMatch(/pair; not replaced file-by-file/);
    expect(screen.queryByTestId("kit-component-replace-socks-file-0-")).toBeNull();
    expect(screen.queryByTestId("kit-component-replace-socks-file-1-")).toBeNull();
    // Pants on the same title are still replaceable.
    expect(screen.getByTestId("kit-component-replace-pants")).toBeTruthy();
  });
});
