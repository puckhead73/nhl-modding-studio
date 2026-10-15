// @vitest-environment jsdom
//
// Phase 2.6 (22 Sep 2026) — the read-only kit-textures card: every required component is listed
// with its evidence and reason, an incomplete kit is never shown as complete, the spare candidates
// carry the same verdict, re-pointing is explained as unsupported, and a late answer for another
// team is never painted as this team's evidence. jsdom only.
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { TeamKitsDto } from "../bindings/TeamKitsDto";

type Pending = { cmd: string; args: any; resolve: (v: any) => void; reject: (e: any) => void };
const calls: Pending[] = [];
vi.mock("../lib/client", () => ({
  call: (cmd: string, args: any) =>
    new Promise((resolve, reject) => {
      calls.push({ cmd, args, resolve, reject });
    }),
}));

// The card now hosts the archive-member replace hook (T2, 24 Sep 2026), which reads the project context.
vi.mock("../lib/project", () => ({ useProject: () => ({ update: () => {} }) }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: () => new Promise(() => {}) }));

import { TeamKitsCard } from "./TeamKitsCard";

const roots = { install_dir: "H:/extracted", platform: "xbox", rendering_override: null, fe_override: null };

function payload(teamId: number): TeamKitsDto {
  return {
    team_id: teamId,
    team_art_id: 7,
    contract_status: "measured",
    sock_grammar: "single_file",
    contract: "jersey `texlib_<style>_<art id>_<variant>.rx2`, pants the same under `pant/`, socks `sock_<style>_<art id>_<variant>_cm — one sock file per kit (NHL 13 → Legacy)` (research R56)",
    rendering_root: "H:/extracted/rendering",
    directories: [
      { part: "jersey", status: "listed", files: 683, reason: null, archive: null },
      { part: "pant", status: "listed", files: 684, reason: null, archive: null },
      { part: "sock", status: "absent", files: null, reason: null, archive: null },
    ],
    kits: [
      {
        record: 10,
        jersey_index: 0,
        style: 0,
        team_art_id: 7,
        variant: 0,
        is_default: true,
        is_light_color: false,
        has_tie: false,
        helmet_logo: true,
        helmet_numbers: false,
        components: [
          { part: "jersey", label: "jersey", file_stem: "texlib_0_7_0", path: "H:/extracted/rendering/jersey/texlib_0_7_0.rx2", rel_path: "rendering/jersey/texlib_0_7_0.rx2", archive_member: null, archive_rel_path: null, member_path: null, status: "present", reason: null },
          { part: "pant", label: "pants", file_stem: "texlib_0_7_0", path: "H:/extracted/rendering/pant/texlib_0_7_0.rx2", rel_path: "rendering/pant/texlib_0_7_0.rx2", archive_member: null, archive_rel_path: null, member_path: null, status: "present", reason: null },
          { part: "sock", label: "socks", file_stem: "sock_0_7_0_cm", path: null, rel_path: null, archive_member: null, archive_rel_path: null, member_path: null, status: "missing", reason: "the `sock/` directory is absent from the rendering tree" },
        ],
        nameplate: { part: "jersey", label: "nameplate", file_stem: "name_0_7_0_cm", path: "H:/extracted/rendering/jersey/name_0_7_0_cm.rx2", rel_path: "rendering/jersey/name_0_7_0_cm.rx2", archive_member: null, archive_rel_path: null, member_path: null, status: "present", reason: null },
        completeness: "incomplete",
        refusal: null,
      },
    ],
    spares: [
      {
        style: 1,
        variant: 9,
        components: [
          { part: "jersey", label: "jersey", file_stem: "texlib_1_7_9", path: "x", rel_path: "rendering/jersey/texlib_1_7_9.rx2", archive_member: null, archive_rel_path: null, member_path: null, status: "present", reason: null },
          { part: "pant", label: "pants", file_stem: "texlib_1_7_9", path: "x", rel_path: "rendering/pant/texlib_1_7_9.rx2", archive_member: null, archive_rel_path: null, member_path: null, status: "present", reason: null },
          { part: "sock", label: "socks", file_stem: "sock_1_7_9_cm", path: null, rel_path: null, archive_member: null, archive_rel_path: null, member_path: null, status: "missing", reason: "the `sock/` directory is absent from the rendering tree" },
        ],
        completeness: "incomplete",
      },
    ],
    reassignment_supported: false,
    reassignment_reason: "re-pointing a row has no write contract yet",
    notes: ["the `sock/` directory is absent: every sock component is missing", `team ${teamId} note`],
  };
}

function mount(teamId: number, style: number | null = 0, variant: number | null = 0) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  const ui = (t: number) => (
    <QueryClientProvider client={qc}>
      <TeamKitsCard projectId={3} teamId={t} rootArgs={roots} installDir="H:/extracted" style={style} variant={variant} />
    </QueryClientProvider>
  );
  const r = render(ui(teamId));
  return { ...r, rerenderTeam: (t: number) => r.rerender(ui(t)) };
}

describe("TeamKitsCard", () => {
  beforeEach(() => {
    calls.length = 0;
  });
  afterEach(cleanup);

  it("lists every required component with its evidence and never calls a sock-less kit complete", async () => {
    mount(5);
    await waitFor(() => expect(calls.length).toBe(1));
    expect(calls[0].cmd).toBe("team_kits");
    expect(calls[0].args).toEqual({ projectId: 3, roots, teamId: 5 });
    calls[0].resolve(payload(5));
    await waitFor(() => expect(screen.getByTestId("kits-card")).toBeTruthy());

    expect(screen.getByTestId("kits-contract-status").textContent).toContain("one sock file per kit");
    expect(screen.getByTestId("kits-directories").textContent).toContain("sock/ absent");
    // The selected row: jersey and pants present, socks missing with the reason, verdict incomplete.
    expect(screen.getByTestId("kits-selected-verdict").textContent).toBe("incomplete");
    expect(screen.getByTestId("kit-component-status-jersey").textContent).toBe("present");
    expect(screen.getByTestId("kit-component-status-pants").textContent).toBe("present");
    expect(screen.getByTestId("kit-component-status-socks").textContent).toBe("missing");
    expect(screen.getByTestId("kit-component-socks").textContent).toContain("directory is absent");
    expect(screen.getByTestId("kit-component-socks").textContent).toContain("sock_0_7_0_cm");
    expect(screen.getByTestId("kits-summary").textContent).toContain("1 kit row: 0 complete · 1 incomplete · 0 unknown");
    // R101 / R103: the nameplate sheet shows beside the kit, present, and does not enter the verdict.
    expect(screen.getByTestId("kits-nameplate").textContent).toContain("name_0_7_0_cm");
    expect(screen.getByTestId("kits-nameplate").textContent).not.toContain("not part of completeness");
    expect(screen.getByTestId("kits-nameplate-status").textContent).toBe("present");
    // The word "complete" never appears as this kit's verdict.
    expect(document.body.textContent).not.toMatch(/\b1 complete\b/);

    // Spares carry the same rule: listed, incomplete, with what is missing.
    const toggle = screen.getByTestId("kits-spares-toggle");
    expect(toggle.textContent).toContain("1 shipped spare candidate no row points at: 0 complete · 1 incomplete · 0 unknown");
    fireEvent.click(toggle);
    await waitFor(() => expect(screen.getByTestId("kits-spare-1-9")).toBeTruthy());
    expect(screen.getByTestId("kits-spare-1-9").textContent).toContain("socks missing");

    expect(screen.getByTestId("kits-reassignment").textContent).toContain("not offered");
    expect(screen.getByTestId("kits-notes").textContent).toContain("every sock component is missing");
  });

  it("shows a selected kit that has no row as a spare candidate with its own verdict", async () => {
    mount(5, 1, 9);
    await waitFor(() => expect(calls.length).toBe(1));
    calls[0].resolve(payload(5));
    await waitFor(() => expect(screen.getByTestId("kits-selected-spare")).toBeTruthy());
    expect(screen.getByTestId("kits-selected-verdict").textContent).toBe("incomplete");
    expect(screen.getByTestId("kit-component-status-socks").textContent).toBe("missing");
  });

  it("never paints a late answer for another team as this team's evidence", async () => {
    const m = mount(1);
    await waitFor(() => expect(calls.length).toBe(1));
    expect(calls[0].args.teamId).toBe(1);
    // The user moves to team 2 before team 1 answers.
    m.rerenderTeam(2);
    await waitFor(() => expect(calls.length).toBe(2));
    expect(calls[1].args.teamId).toBe(2);
    // Team 1's late answer arrives first: nothing is painted for team 2.
    calls[0].resolve(payload(1));
    await new Promise((r) => setTimeout(r, 20));
    expect(screen.queryByTestId("kits-card")).toBeNull();
    // Team 2's own answer arrives: painted, and it says team 2.
    calls[1].resolve(payload(2));
    await waitFor(() => expect(screen.getByTestId("kits-card")).toBeTruthy());
    expect(screen.getByTestId("kits-card").getAttribute("data-team-id")).toBe("2");
    expect(screen.getByTestId("kits-notes").textContent).toContain("team 2 note");
    expect(screen.getByTestId("kits-notes").textContent).not.toContain("team 1 note");
  });

  it("treats a payload that names a different team as still loading", async () => {
    mount(4);
    await waitFor(() => expect(calls.length).toBe(1));
    calls[0].resolve(payload(9)); // a mismatched result is not this team's evidence
    await new Promise((r) => setTimeout(r, 20));
    expect(screen.queryByTestId("kits-card")).toBeNull();
  });

  it("shows the backend's error instead of an empty card", async () => {
    mount(5);
    await waitFor(() => expect(calls.length).toBe(1));
    calls[0].reject(new Error("the open project has no recorded title"));
    await waitFor(() => expect(screen.getByTestId("kits-error").textContent).toContain("no recorded title"));
  });
});
