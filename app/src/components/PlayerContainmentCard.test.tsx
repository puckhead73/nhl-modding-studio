// @vitest-environment jsdom
//
// Phase 3 (22 Sep 2026) — the read-only containment card: the summary shows the cross-tab as data
// with the flag never reworded as a file check, the residual names are listed, a player row shows
// its ids / flag / evidence / note, an unmeasured title says so, and the query is keyed on the
// install root so a different root is a different answer. jsdom only.
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { PlayerContainmentViewDto } from "../bindings/PlayerContainmentViewDto";

const calls: Array<{ cmd: string; args: any }> = [];
let payload: PlayerContainmentViewDto | null = null;
let fail: string | null = null;
let reassign: ((args: any) => unknown) | null = null;
vi.mock("../lib/client", () => ({
  call: (cmd: string, args: any) => {
    calls.push({ cmd, args });
    if (cmd === "player_reassign" && reassign) return Promise.resolve().then(() => reassign!(args));
    return fail ? Promise.reject(new Error(fail)) : Promise.resolve(payload);
  },
}));

import { PlayerContainmentCard } from "./PlayerContainmentCard";

const roots = { install_dir: "X:/legacy", platform: "xbox", rendering_override: null, fe_override: null };

function view(): PlayerContainmentViewDto {
  return {
    summary: {
      contract_status: "gen7",
      contract: "portrait … head … (research R53)",
      players: 5632,
      bespoke: 246,
      bespoke_head_present: 246,
      bespoke_portrait_present: 246,
      pool_players: 5385,
      pool_head_present: 5385,
      pool_floor: 60000,
      flag1_file1: 4376,
      flag1_file0: 2,
      flag0_file1: 127,
      flag0_file0: 1127,
      has_flag_column: true,
      shipped_portraits: 9224,
      shipped_heads: 904,
      unused_portraits: 4724,
      unused_heads: 457,
      bespoke_missing_head: [],
      bespoke_missing_portrait: [],
      flagged_without_file: ["Ladislav Nagy", "Someone Else"],
      notes: ["5632 players: …", "`hasportrait` vs the shipped file: … never written from them", "read-only: …"],
    },
    players: [
      { bio_record: 7, full_name: "Ladislav Nagy", portrait_id: 1234, head_id: 60010, has_portrait_flag: true, class: "pool", portrait: "missing", head: "present", note: "`hasportrait` is set but no portrait file ships for art id 1234 — the flag is shown as stored, not corrected" },
      { bio_record: 8, full_name: "Star Player", portrait_id: 22, head_id: 22, has_portrait_flag: true, class: "bespoke", portrait: "present", head: "present", note: null },
    ],
  };
}

function mount(bioRecord?: number, installDir = "X:/legacy") {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <PlayerContainmentCard projectId={2} rootArgs={roots} installDir={installDir} bioRecord={bioRecord} />
    </QueryClientProvider>,
  );
}

describe("PlayerContainmentCard", () => {
  beforeEach(() => {
    reassign = null;
    calls.length = 0;
    payload = view();
    fail = null;
  });
  afterEach(cleanup);

  it("shows the cross-tab as data, the pools and the residual names", async () => {
    mount();
    await waitFor(() => expect(screen.getByTestId("containment-summary")).toBeTruthy());
    expect(calls[0]).toEqual({ cmd: "player_containment", args: { projectId: 2, roots } });
    expect(screen.getByTestId("containment-contract-status").textContent).toContain("gen7");
    expect(screen.getByTestId("containment-bespoke").textContent).toContain("246 — head file 246, portrait file 246");
    expect(screen.getByTestId("containment-pool").textContent).toContain("5385 — head file 5385");
    expect(screen.getByTestId("containment-flag").textContent).toContain("flagged with file 4376 · flagged without 2 · clear with file 127 · clear without 1127");
    expect(screen.getByTestId("containment-unused").textContent).toContain("portraits 4724 of 9224 · heads 457 of 904");
    expect(screen.getByTestId("containment-residuals").textContent).toContain("Flagged as having a portrait, no file: Ladislav Nagy, Someone Else");
    // The card never claims to have fixed or derived the flag.
    expect(document.body.textContent).toContain("never derived from a file and never written from one");
    expect(document.body.textContent).not.toMatch(/corrected the flag|flag updated/i);
  });

  it("shows one player's ids, flag, evidence and note", async () => {
    mount(7);
    await waitFor(() => expect(screen.getByTestId("containment-player")).toBeTruthy());
    expect(screen.getByTestId("containment-player").getAttribute("data-bio-record")).toBe("7");
    expect(screen.getByTestId("containment-player-class").textContent).toBe("pool head");
    expect(screen.getByTestId("containment-player-portrait").textContent).toBe("missing");
    expect(screen.getByTestId("containment-player-head").textContent).toBe("present");
    expect(screen.getByTestId("containment-player-flag").textContent).toBe("hasportrait = 1 (as stored)");
    expect(screen.getByTestId("containment-player-note").textContent).toContain("shown as stored, not corrected");
    cleanup();
    mount(8);
    await waitFor(() => expect(screen.getByTestId("containment-player")).toBeTruthy());
    expect(screen.getByTestId("containment-player-class").textContent).toBe("own head (head id = art id)");
    expect(screen.queryByTestId("containment-player-note")).toBeNull();
    cleanup();
    mount(99);
    await waitFor(() => expect(screen.getByTestId("containment-player-none")).toBeTruthy());
  });

  it("checks a reassignment target, shows the evidence or the refusal, and applies only an allowed plan (receipt 38)", async () => {
    const plan = (a: any, allowed: boolean) => ({
      kind: a.kind,
      bio_record: a.bioRecord,
      player: "Star Player",
      current_id: 22,
      target_id: a.target,
      allowed,
      refusals: allowed ? [] : [`no portrait file p${a.target}.big ships in this title's playerheads tree`],
      evidence: allowed ? [`X:/legacy/fe/ion/artassets/playerheads/p0_4000/p${a.target}.big`] : [],
      notes: ["`hasportrait` stays as stored (1); Studio never derives or flips it from a file"],
      source_identity: [],
      written_fields: ["lOnr"],
      fingerprint: `fp-${a.target}`,
      applied: a.apply ? { edits: 1, mirrored: 0 } : null,
    });
    reassign = (a) => plan(a, a.target !== 99999);
    mount(8);
    await screen.findByTestId("reassign-portrait");
    const input = screen.getByTestId("reassign-portrait-target") as HTMLInputElement;
    const apply = screen.getByTestId("reassign-portrait-apply") as HTMLButtonElement;
    expect(apply.disabled).toBe(true);
    fireEvent.change(input, { target: { value: "99999" } });
    fireEvent.click(screen.getByTestId("reassign-portrait-check"));
    await screen.findByTestId("reassign-portrait-refusal");
    expect(screen.getByTestId("reassign-portrait-refusal").textContent).toMatch(/no portrait file p99999\.big/);
    expect(apply.disabled).toBe(true);
    fireEvent.change(input, { target: { value: "31" } });
    expect(screen.queryByTestId("reassign-portrait-plan")).toBeNull();
    fireEvent.click(screen.getByTestId("reassign-portrait-check"));
    await screen.findByText(/ships: .*p31\.big/);
    expect(apply.disabled).toBe(false);
    fireEvent.click(apply);
    await screen.findByTestId("reassign-portrait-done");
    const sent = calls.filter((c) => c.cmd === "player_reassign").map((c) => ({ kind: c.args.kind, target: c.args.target, apply: c.args.apply, bioRecord: c.args.bioRecord, reviewed: c.args.reviewed }));
    // Receipt 63: the apply names the fingerprint of the plan that was checked.
    expect(sent).toEqual([
      { kind: "portrait", target: 99999, apply: false, bioRecord: 8, reviewed: null },
      { kind: "portrait", target: 31, apply: false, bioRecord: 8, reviewed: null },
      { kind: "portrait", target: 31, apply: true, bioRecord: 8, reviewed: "fp-31" },
    ]);
    expect(screen.getByTestId("reassign-portrait-done").textContent).toMatch(/staged as one edit.*Pending changes/);
    expect(screen.getByTestId("reassign-head")).toBeTruthy();
    // Hair is a PS4 row only.
    expect(screen.queryByTestId("reassign-hair")).toBeNull();
  });

  it("offers PS4 hair with the stored value, the vault evidence and the exact checked source (receipt 66)", async () => {
    payload = { ...view(), summary: { ...view().summary, contract_status: "ps4" } };
    reassign = (a: any) => ({
      kind: "hair",
      bio_record: a.bioRecord,
      player: "Star Player",
      current_id: 0,
      target_id: a.target,
      allowed: true,
      refusals: [],
      evidence: [`ge_nhl_skaterhairtexturepack with GenericHairStyle ${a.target} — nhl21 vault in databigs/boot.big (gccollections.vlt 0123456789ab…)`],
      notes: ["only `hairid` is written: `headid`, `headtexture` and the other appearance fields stay as stored"],
      source_identity: ["databigs/boot.big: gccollections.vlt sha256 " + "ab".repeat(32), "defs/ps4/vault-identities.json sha256 " + "cd".repeat(32)],
      written_fields: ["RAXG"],
      fingerprint: `hair-${a.target}`,
      applied: a.apply ? { edits: 1, mirrored: 0 } : null,
    });
    mount(8);
    await screen.findByTestId("reassign-hair");
    fireEvent.change(screen.getByTestId("reassign-hair-target"), { target: { value: "5" } });
    fireEvent.click(screen.getByTestId("reassign-hair-check"));
    await screen.findByText(/ships: ge_nhl_skaterhairtexturepack with GenericHairStyle 5/);
    expect(screen.getByTestId("reassign-hair").textContent).toContain("Reassign hair style (0) to");
    expect(screen.getByTestId("reassign-hair-source").textContent).toContain("ab".repeat(32));
    fireEvent.click(screen.getByTestId("reassign-hair-apply"));
    await screen.findByTestId("reassign-hair-done");
    expect(screen.getByTestId("reassign-hair-done").textContent).toMatch(/^Hair style set to 5/);
    const sent = calls.filter((c) => c.cmd === "player_reassign").map((c) => ({ kind: c.args.kind, target: c.args.target, apply: c.args.apply, reviewed: c.args.reviewed }));
    expect(sent).toEqual([
      { kind: "hair", target: 5, apply: false, reviewed: null },
      { kind: "hair", target: 5, apply: true, reviewed: "hair-5" },
    ]);
  });

  it("says when the title is not measured and shows the backend error", async () => {
    payload = { summary: { ...view().summary, contract_status: "unmeasured", has_flag_column: false, notes: ["the portrait / head containment of `nhl08` is not measured …"] }, players: [] };
    mount();
    await waitFor(() => expect(screen.getByTestId("containment-contract-status").textContent).toContain("not measured"));
    expect(screen.queryByTestId("containment-table")).toBeNull();
    cleanup();
    fail = "the open database has no player bio table";
    mount();
    await waitFor(() => expect(screen.getByTestId("containment-error").textContent).toContain("no player bio table"));
  });

  it("keys the query on the install root", async () => {
    mount(undefined, "X:/one");
    await waitFor(() => expect(calls.length).toBe(1));
    cleanup();
    mount(undefined, "X:/two");
    await waitFor(() => expect(calls.length).toBe(2));
  });
});
