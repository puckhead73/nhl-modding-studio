// @vitest-environment jsdom
//
// ONB-003 §5 — the installer page: a package is read and validated first, the destination is
// chosen among the package's declared consoles only, the preflight is read-only and shows the exact
// planned changes and conflicts, and Install is sent only for an installable plan. Component
// evidence only (jsdom; commands mocked and their exact arguments asserted); the installer's
// staging, transaction, backups and restore are the Rust suite `mod_package`.
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { MemoryRouter } from "react-router-dom";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { PackageInspectDto } from "../bindings/PackageInspectDto";
import type { InstallPlanDto } from "../bindings/InstallPlanDto";

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
vi.mock("../lib/project", () => ({ useProject: () => ({ project: null }) }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));

import { InstallPackagePage } from "./InstallPackage";

const PKG = "D:\\Downloads\\Legacy Nordic-1.0.nhlmod";
const ROOT = "C:\\RPCS3";

function inspected(): PackageInspectDto {
  return {
    path: PKG,
    author: "Test author",
    len: 4096,
    sha256: "abc123".padEnd(64, "0"),
    package_id: "pkgid".padEnd(64, "0"),
    name: "Legacy Nordic",
    version: "1.0",
    description: "one cell and one dictionary text",
    created: "2026-10-01T12:00:00Z",
    studio_version: "0.1.0",
    game_id: "nhllegacy",
    title: "NHL Legacy",
    author_platform: "xbox",
    targets: [
      { platform: "xbox", status: "supported", components: [{ category: "cells", status: "supported", reason: "author" }] },
      { platform: "ps3", status: "supported", components: [{ category: "cells", status: "supported", reason: "byte-identical baseline" }] },
      { platform: "recomp", status: "unsupported", components: [{ category: "textures", status: "unsupported", reason: "paths are not mapped" }] },
    ],
    base: [],
    categories: [{ category: "cells", count: 1 }, { category: "dictionary", count: 1 }, { category: "textures", count: 0 }],
    entries: [{ path: "edits.json", len: 900, sha256: "e".repeat(64), kind: "edits" }],
    cells: [{ file: "both", table_name: "Players", record: 2, entity: "A Player", field_name: "position", before: "C", after: "LW" }],
    dictionary: [{ key: "TXT_LEAGUE_NORWAY", before: "Norway", after: "Nordic" }],
    textures: [],
    changes: [],
    project_source_kind: "fresh",
    edits_schema: "nhlms-modpkg-edits/2",
  };
}

function plan(over: Partial<InstallPlanDto> = {}): InstallPlanDto {
  return {
    handle: 11,
    installable: true,
    reasons: [],
    conflicts: [],
    requirements: [{ role: "main_db", status: "found", detail: `${ROOT}\\dev_hdd0\\game\\BLUS31540\\USRDIR\\data\\db\\nhlng.db` }],
    items: [
      { key: "main_db", role: "database", label: "Main database (nhlng.db)", destination_path: `${ROOT}\\…\\nhlng.db`, destination: { len: 1, sha256: "a".repeat(64) }, replacement: { len: 1, sha256: "b".repeat(64) }, state: "replace", notes: [] },
      { key: "roster", role: "roster-save", label: "Roster save (SYS-DATA)", destination_path: `${ROOT}\\…\\SYS-DATA`, destination: { len: 1, sha256: "c".repeat(64) }, replacement: { len: 1, sha256: "d".repeat(64) }, state: "replace", notes: [] },
    ],
    applied: 2,
    already_present: 0,
    backup_bytes: 2,
    replacement_bytes: 2,
    package_name: "Legacy Nordic",
    package_version: "1.0",
    package_id: "pkgid".padEnd(64, "0"),
    game_id: "nhllegacy",
    platform: "ps3",
    records_dir: "C:\\Users\\x\\AppData\\…\\installs",
    ...over,
  };
}

function renderPage(state?: { packagePath: string; gameId: string; platform: string; root: string }) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <MemoryRouter initialEntries={[{ pathname: "/manager/install", state }]}>
        <InstallPackagePage />
      </MemoryRouter>
    </QueryClientProvider>,
  );
}
const lastCall = (cmd: string) => [...calls].reverse().find((c) => c.cmd === cmd);

beforeEach(() => {
  calls.length = 0;
  for (const k of Object.keys(handlers)) delete handlers[k];
  handlers.package_installs = () => [];
  handlers.package_inspect = () => inspected();
  handlers.detect_install = () => ({ game_id: "nhllegacy", platform: "ps3", picked_root: ROOT, install_dir: `${ROOT}\\x`, nhlng_path: `${ROOT}\\x\\nhlng.db`, localization_path: `${ROOT}\\x\\nhl_eng_us.db`, meta_xml_path: `${ROOT}\\x\\nhlng-meta.xml`, saves: [], minimum_met: false, extract_needed: false, has_existing_profile: false, game_dir: null, save_root: null, game_candidates: [], problems: [] });
  handlers.package_preflight = () => plan();
  handlers.package_apply = () => ({ seq: 1, state: "committed", detail: null, record_dir: "C:\\records\\1", destinations: [] });
});
afterEach(() => cleanup());

describe("ONB-003 installer", () => {
  it("uses the manager's selected package and Xbox target without requiring a save folder or installing automatically", async () => {
    handlers.package_inspect = () => ({ ...inspected(), cells: inspected().cells.map(c => ({ ...c, file: "main_db" })) });
    renderPage({ packagePath: PKG, gameId: "nhllegacy", platform: "xbox", root: ROOT });
    await screen.findByTestId("package-summary");
    await waitFor(() => expect(lastCall("detect_install")?.args).toEqual({ gameId: "nhllegacy", platform: "xbox", root: ROOT, savePath: null }));
    expect((screen.getByLabelText("Package file") as HTMLInputElement).value).toBe(PKG);
    expect(lastCall("package_preflight")).toBeUndefined();
    expect(lastCall("package_apply")).toBeUndefined();
  });
  it("reads the package first, offers only its supported consoles, previews the plan and installs only on Install", async () => {
    renderPage();
    fireEvent.change(screen.getByLabelText("Package file"), { target: { value: PKG } });
    fireEvent.click(screen.getByRole("button", { name: "Read package" }));
    await waitFor(() => expect(lastCall("package_inspect")).toBeTruthy());
    expect(lastCall("package_inspect")!.args).toEqual({ path: PKG });
    await screen.findByTestId("package-summary");
    expect(screen.getByText(/Legacy Nordic/)).toBeTruthy();
    expect(screen.getByText(/Players › A Player › position: C → LW/)).toBeTruthy();
    // Two supported consoles, one not: the unsupported one cannot be picked.
    const radios = screen.getAllByRole("radio", { name: /PS3|Xbox 360|Recompiled/ }) as HTMLInputElement[];
    expect(radios.length).toBe(3);
    const recomp = radios.find((r) => r.parentElement?.textContent?.includes("Recompiled"))!;
    expect(recomp.disabled).toBe(true);
    fireEvent.click(radios.find((r) => r.parentElement?.textContent?.includes("PS3"))!);
    fireEvent.change(screen.getByLabelText("Game installation folder"), { target: { value: ROOT } });
    await waitFor(() => expect(lastCall("detect_install")).toBeTruthy());
    expect(lastCall("detect_install")!.args).toEqual({ gameId: "nhllegacy", platform: "ps3", root: ROOT, savePath: null });
    await screen.findByTestId("destination-report");
    // Nothing was planned or written yet.
    expect(lastCall("package_preflight")).toBeUndefined();
    fireEvent.click(screen.getByRole("button", { name: "Review installation" }));
    await waitFor(() => expect(lastCall("package_preflight")).toBeTruthy());
    expect(lastCall("package_preflight")!.args).toEqual({ packagePath: PKG, gameId: "nhllegacy", platform: "ps3", root: ROOT, savePath: null, rosterSave: null, allowConflicts: false });
    await screen.findByTestId("install-plan");
    expect(screen.getByText(/2 edit\(s\) to apply/)).toBeTruthy();
    expect(screen.getAllByText(/replaced \(backed up first\)/).length).toBe(2);
    expect(lastCall("package_apply")).toBeUndefined();
    fireEvent.click(screen.getByRole("button", { name: "Install" }));
    await waitFor(() => expect(lastCall("package_apply")).toBeTruthy());
    expect(lastCall("package_apply")!.args).toEqual({ handle: 11 });
    await screen.findByTestId("install-result");
  });

  it("a conflicting plan cannot be installed until conflicts are explicitly allowed, and a refused package shows its reason", async () => {
    handlers.package_preflight = (a: any) =>
      a.allowConflicts
        ? plan({ conflicts: ["Players › A Player › position: the destination holds RW where the package expected C — replacing it as allowed"] })
        : plan({ installable: false, conflicts: ["Players › A Player › position: the destination holds RW where the package expected C (the recipient's own change)"], reasons: ["1 conflict(s) with the recipient's own changes; review them and allow replacing conflicting values to continue"] });
    renderPage();
    fireEvent.change(screen.getByLabelText("Package file"), { target: { value: PKG } });
    fireEvent.click(screen.getByRole("button", { name: "Read package" }));
    await screen.findByTestId("package-summary");
    const radios = screen.getAllByRole("radio", { name: /PS3|Xbox 360|Recompiled/ }) as HTMLInputElement[];
    fireEvent.click(radios.find((r) => r.parentElement?.textContent?.includes("Xbox 360"))!);
    fireEvent.change(screen.getByLabelText("Game installation folder"), { target: { value: "C:\\Games\\Legacy" } });
    fireEvent.change(screen.getByLabelText("Save-data location"), { target: { value: "C:\\Xenia\\saves" } });
    fireEvent.click(screen.getByRole("button", { name: "Review installation" }));
    await screen.findByTestId("install-plan");
    expect(screen.getAllByText(/the recipient's own change/).length).toBeGreaterThan(0);
    expect((screen.getByRole("button", { name: "Install" }) as HTMLButtonElement).disabled).toBe(true);
    fireEvent.click(screen.getByLabelText(/Allow replacing values/));
    // Changing the choice drops the stale plan; a new review is explicit.
    expect(screen.queryByTestId("install-plan")).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Review installation" }));
    await screen.findByTestId("install-plan");
    expect(lastCall("package_preflight")!.args.allowConflicts).toBe(true);
    expect((screen.getByRole("button", { name: "Install" }) as HTMLButtonElement).disabled).toBe(false);

    cleanup();
    calls.length = 0;
    handlers.package_inspect = () => {
      throw new Error("the package is not a valid Studio mod package: entry \"payloads/x.png\" fails its CRC check; the package is damaged");
    };
    renderPage();
    fireEvent.change(screen.getByLabelText("Package file"), { target: { value: PKG } });
    fireEvent.click(screen.getByRole("button", { name: "Read package" }));
    await screen.findByText(/fails its CRC check/);
    expect(screen.queryByTestId("package-summary")).toBeNull();
    expect(calls.some((c) => c.cmd === "package_preflight" || c.cmd === "package_apply")).toBe(false);
  });
});

describe("R71 corrections — roles, explicit roster choice, recovery", () => {
  it("a database-only package needs no roster: no choice is asked even when several saves exist, and the plan says the roster is not needed", async () => {
    handlers.package_inspect = () => ({ ...inspected(), cells: [{ file: "main_db", table_name: "Players", record: 2, entity: "A Player", field_name: "position", before: "C", after: "LW" }] });
    handlers.detect_install = () => ({ game_id: "nhllegacy", platform: "ps3", picked_root: ROOT, install_dir: `${ROOT}\\x`, nhlng_path: `${ROOT}\\x\\nhlng.db`, localization_path: null, meta_xml_path: `${ROOT}\\x\\nhlng-meta.xml`, saves: [
      { kind: "roster", path: `${ROOT}\\saves\\A\\SYS-DATA`, container_version: "v2", writable: true },
      { kind: "roster", path: `${ROOT}\\saves\\B\\SYS-DATA`, container_version: "v2", writable: true },
    ], minimum_met: true, extract_needed: false, has_existing_profile: false, game_dir: null, save_root: null, game_candidates: [], problems: [] });
    handlers.package_preflight = () => plan({
      requirements: [
        { role: "main_db", status: "found", detail: `${ROOT}\\x\\nhlng.db` },
        { role: "roster_save", status: "not-needed", detail: "the package has no roster-save edit; no save is touched" },
      ],
      items: [plan().items[0]],
      applied: 1,
    });
    renderPage();
    fireEvent.change(screen.getByLabelText("Package file"), { target: { value: PKG } });
    fireEvent.click(screen.getByRole("button", { name: "Read package" }));
    await screen.findByTestId("package-summary");
    const radios = screen.getAllByRole("radio", { name: /PS3|Xbox 360|Recompiled/ }) as HTMLInputElement[];
    fireEvent.click(radios.find((r) => r.parentElement?.textContent?.includes("PS3"))!);
    fireEvent.change(screen.getByLabelText("Game installation folder"), { target: { value: ROOT } });
    await screen.findByTestId("destination-report");
    expect(screen.queryByTestId("roster-choice")).toBeNull();
    expect(screen.getByText(/edits no roster save; none is touched/)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Review installation" }));
    await screen.findByTestId("install-plan");
    // Two saves exist but the package names none: the request sends no roster at all.
    expect(lastCall("package_preflight")!.args.rosterSave).toBeNull();
    expect(screen.getByText(/roster save · not needed/)).toBeTruthy();
    expect(screen.getByText(/no save is touched/)).toBeTruthy();
  });

  it("a package with roster edits on PS3 asks which of several roster saves receives them and sends exactly that choice", async () => {
    handlers.package_inspect = () => ({ ...inspected(), cells: [{ file: "both", table_name: "Players", record: 2, entity: "A Player", field_name: "position", before: "C", after: "LW" }] });
    const a = `${ROOT}\\saves\\A\\SYS-DATA`;
    const b = `${ROOT}\\saves\\B\\SYS-DATA`;
    handlers.detect_install = () => ({ game_id: "nhllegacy", platform: "ps3", picked_root: ROOT, install_dir: `${ROOT}\\x`, nhlng_path: `${ROOT}\\x\\nhlng.db`, localization_path: null, meta_xml_path: `${ROOT}\\x\\nhlng-meta.xml`, saves: [
      { kind: "roster", path: a, container_version: "v2", writable: true },
      { kind: "roster", path: b, container_version: "v2", writable: true },
    ], minimum_met: true, extract_needed: false, has_existing_profile: false, game_dir: null, save_root: null, game_candidates: [], problems: [] });
    renderPage();
    fireEvent.change(screen.getByLabelText("Package file"), { target: { value: PKG } });
    fireEvent.click(screen.getByRole("button", { name: "Read package" }));
    await screen.findByTestId("package-summary");
    const radios = screen.getAllByRole("radio", { name: /PS3|Xbox 360|Recompiled/ }) as HTMLInputElement[];
    fireEvent.click(radios.find((r) => r.parentElement?.textContent?.includes("PS3"))!);
    fireEvent.change(screen.getByLabelText("Game installation folder"), { target: { value: ROOT } });
    await screen.findByTestId("destination-report");
    await screen.findByTestId("roster-choice");
    // Until a save is chosen the review cannot run; nothing is picked silently.
    expect((screen.getByRole("button", { name: "Review installation" }) as HTMLButtonElement).disabled).toBe(true);
    const choice = Array.from(screen.getByTestId("roster-choice").querySelectorAll('input[name="roster"]')) as HTMLInputElement[];
    expect(choice.length).toBe(2);
    fireEvent.click(choice[1]);
    expect((screen.getByRole("button", { name: "Review installation" }) as HTMLButtonElement).disabled).toBe(false);
    fireEvent.click(screen.getByRole("button", { name: "Review installation" }));
    await screen.findByTestId("install-plan");
    expect(lastCall("package_preflight")!.args.rosterSave).toBe(b);
  });

  it("an interrupted installation is offered as Recover and sent to package_restore", async () => {
    handlers.package_installs = () => [
      { record_dir: "C:\\records\\7", seq: 7, at: "2026-10-01T20:00:00Z", state: "restore-incomplete", package_name: "Legacy Nordic", package_version: "1.0", package_id: "p".repeat(64), game_id: "nhllegacy", platform: "recomp", picked_root: "C:\\Recomp", restorable: true, recovery: true, items: [] },
      { record_dir: "C:\\records\\6", seq: 6, at: "2026-10-01T19:00:00Z", state: "restored", package_name: "Legacy Nordic", package_version: "1.0", package_id: "p".repeat(64), game_id: "nhllegacy", platform: "recomp", picked_root: "C:\\Recomp", restorable: false, recovery: false, items: [] },
    ];
    handlers.package_restore = () => ({ seq: 7, state: "restored", detail: null, record_dir: "C:\\records\\7", destinations: [{ key: "main_db", label: "Main database", path: "x", state: "prior", backup: null }] });
    renderPage();
    const recover = await screen.findByRole("button", { name: "Recover" });
    expect(screen.queryByRole("button", { name: "Restore originals" })).toBeNull();
    fireEvent.click(recover);
    await waitFor(() => expect(lastCall("package_restore")).toBeTruthy());
    expect(lastCall("package_restore")!.args).toEqual({ recordDir: "C:\\records\\7" });
    await screen.findByText(/Restored: 1 destination/);
  });
});

describe("R72-R1 — roster-only package without a game installation", () => {
  it("submits the review with an empty game root, the save-data location and the explicit save", async () => {
    handlers.package_inspect = () => ({ ...inspected(), author_platform: "xbox", targets: [{ platform: "xbox", status: "supported", components: [{ category: "cells", status: "supported", reason: "author" }] }], cells: [{ file: "roster", table_name: "Players", record: 2, entity: "A Player", field_name: "position", before: "C", after: "LW" }], dictionary: [], categories: [{ category: "cells", count: 1 }, { category: "dictionary", count: 0 }, { category: "textures", count: 0 }] });
    const saves = "D:\\Xenia\\saves";
    const a = `${saves}\\ROSTER A\\ROSTER A`;
    const b = `${saves}\\ROSTER B\\ROSTER B`;
    handlers.detect_install = () => ({ game_id: "nhllegacy", platform: "xbox", picked_root: "", install_dir: "", nhlng_path: null, localization_path: null, meta_xml_path: null, saves: [
      { kind: "roster", path: a, container_version: "v2", writable: true },
      { kind: "roster", path: b, container_version: "v2", writable: true },
    ], minimum_met: false, extract_needed: false, has_existing_profile: false, game_dir: null, save_root: saves, game_candidates: [], problems: [] });
    handlers.package_preflight = () => plan({
      platform: "xbox",
      requirements: [
        { role: "main_db", status: "not-needed", detail: "the package has no main-database edit; the database is not touched" },
        { role: "schema", status: "found", detail: "bundled nhlng-meta.xml (verified-vanilla, 6c335a62d1a0…)" },
        { role: "roster_save", status: "found", detail: b },
      ],
      items: [{ key: "roster", role: "roster-save", label: "Roster save (ROSTER B)", destination_path: b, destination: { len: 1, sha256: "c".repeat(64) }, replacement: { len: 1, sha256: "d".repeat(64) }, state: "replace", notes: [] }],
      applied: 1,
    });
    renderPage();
    fireEvent.change(screen.getByLabelText("Package file"), { target: { value: PKG } });
    fireEvent.click(screen.getByRole("button", { name: "Read package" }));
    await screen.findByTestId("package-summary");
    // One supported console: picked automatically; the game folder is optional for this package.
    await screen.findByTestId("roster-only-note");
    expect((screen.getByRole("button", { name: "Review installation" }) as HTMLButtonElement).disabled).toBe(true);
    fireEvent.change(screen.getByLabelText("Save-data location"), { target: { value: saves } });
    await waitFor(() => expect(lastCall("detect_install")).toBeTruthy());
    expect(lastCall("detect_install")!.args).toEqual({ gameId: "nhllegacy", platform: "xbox", root: "", savePath: saves });
    await screen.findByTestId("roster-choice");
    expect(screen.getByText(/Database and dictionary: not needed/)).toBeTruthy();
    const choice = Array.from(screen.getByTestId("roster-choice").querySelectorAll('input[name="roster"]')) as HTMLInputElement[];
    fireEvent.click(choice[1]);
    const review = screen.getByRole("button", { name: "Review installation" }) as HTMLButtonElement;
    expect(review.disabled).toBe(false);
    fireEvent.click(review);
    await waitFor(() => expect(lastCall("package_preflight")).toBeTruthy());
    expect(lastCall("package_preflight")!.args).toEqual({ packagePath: PKG, gameId: "nhllegacy", platform: "xbox", root: "", savePath: saves, rosterSave: b, allowConflicts: false });
    await screen.findByTestId("install-plan");
    expect(screen.getByText(/bundled nhlng-meta.xml/)).toBeTruthy();
    expect(screen.getByText(/main db · not needed/)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Install" }));
    await waitFor(() => expect(lastCall("package_apply")).toBeTruthy());
    expect(lastCall("package_apply")!.args).toEqual({ handle: 11 });
  });

  it("a package that also edits the database still needs the game installation folder", async () => {
    handlers.package_inspect = () => ({ ...inspected(), author_platform: "xbox", targets: [{ platform: "xbox", status: "supported", components: [{ category: "cells", status: "supported", reason: "author" }] }], cells: [{ file: "both", table_name: "Players", record: 2, entity: "A Player", field_name: "position", before: "C", after: "LW" }], dictionary: [] });
    renderPage();
    fireEvent.change(screen.getByLabelText("Package file"), { target: { value: PKG } });
    fireEvent.click(screen.getByRole("button", { name: "Read package" }));
    await screen.findByTestId("package-summary");
    await screen.findByLabelText("Game installation folder");
    expect(screen.queryByTestId("roster-only-note")).toBeNull();
    fireEvent.change(screen.getByLabelText("Save-data location"), { target: { value: "D:\\Xenia\\saves" } });
    expect((screen.getByRole("button", { name: "Review installation" }) as HTMLButtonElement).disabled).toBe(true);
    expect(lastCall("detect_install")).toBeUndefined();
  });
});
