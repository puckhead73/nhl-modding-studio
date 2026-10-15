// @vitest-environment jsdom
//
// ONB-002 onboarding: source choice + project folder → game → console → paths → extraction (only
// when needed) → Studio. Component evidence only (jsdom): the backend commands are mocked and the
// assertions are on the exact arguments sent and the stored settings. Project creation, isolation,
// saving, deployment and extraction are the Rust suites' job (`studio-api` tests
// `project_workspace`, `ps4_onboarding_paths`; `studio-domain` project_store / extract_service).
//
// The r69 PS4 guards are preserved on the path screen: a remembered roster save that disappeared is
// never replaced silently, several roster saves need a choice, and problems stay distinct.
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { open as pickDialog, confirm as confirmDialog } from "@tauri-apps/plugin-dialog";
import { MemoryRouter } from "react-router-dom";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { InstallReportDto } from "../bindings/InstallReportDto";
import type { SaveSlotDto } from "../bindings/SaveSlotDto";
import type { ProjectOpenDto } from "../bindings/ProjectOpenDto";
import type { ExtractPreflightDto } from "../bindings/ExtractPreflightDto";

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
const opened: unknown[] = [];
vi.mock("../lib/project", () => ({
  useProject: () => ({ project: null, open: (...a: unknown[]) => opened.push(a) }),
}));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn(), confirm: vi.fn() }));
vi.mock("../components/IsoOpenPanel", () => ({ IsoOpenPanel: () => null }));
vi.mock("../components/LingeringProjectNotice", () => ({ LingeringProjectNotice: () => null }));

import { OpenProject } from "./OpenProject";
import { SettingsProvider } from "../lib/settings";

const SUGGESTED = "C:\\Users\\tester\\AppData\\Roaming\\shadPS4\\savedata";
const GAME = "D:\\PS4 Games\\hockey twenty-one";
const PROJECT = "D:\\Studio Projects\\ps4\\nhl21\\nhl21-fresh";
const STORAGE_KEY = "nhl-modding-studio.settings.v1";

function roster(profile: string, save = "RosterCustom0"): SaveSlotDto {
  return {
    kind: "roster",
    label: "Roster",
    path: `${SUGGESTED}\\${profile}\\CUSA18182\\${save}\\DATA`,
    container_version: "ps4",
    writable: true,
    profile,
    title_id: "CUSA18182",
    problem: null,
  };
}

function report(over: Partial<InstallReportDto> = {}): InstallReportDto {
  return {
    game_id: "nhl21",
    platform: "ps4",
    picked_root: GAME,
    install_dir: `${GAME}\\nhl\\data\\common\\gamemodes\\database\\t3db`,
    nhlng_path: `${GAME}\\nhl\\data\\common\\gamemodes\\database\\t3db\\nhl_game.db`,
    localization_path: null,
    meta_xml_path: `${GAME}\\nhl\\data\\common\\gamemodes\\database\\t3db\\nhl_game-meta.xml`,
    saves: [roster("1"), roster("2")],
    minimum_met: true,
    extract_needed: false,
    has_existing_profile: false,
    game_dir: GAME,
    save_root: SUGGESTED,
    game_candidates: [
      { path: GAME, title: "NHL™ 21", title_id: "CUSA18182", content_id: "EP0006-CUSA18182_00-NHLICEHOCKEY2021", app_version: "01.00", matches: true, reason: null },
    ],
    problems: [],
    ...over,
  };
}

function inventory(status = "partial") {
  return {
    schema: "nhlms-baselines/1",
    head: "test",
    inventory_path: "x",
    files_root: "y",
    targets: [
      { game_id: "nhl21", platform: "ps4", title: "NHL 21 (PS4)", status, components: [], gaps: status === "available" ? [] : [{ component: "dictionary", reason: "no eng_us.db is committed for NHL 21" }, { component: "roster_save", reason: "no vanilla roster save is committed" }] },
      { game_id: "nhl20", platform: "ps4", title: "NHL 20 (PS4)", status: "unavailable", components: [], gaps: [{ component: "main_db", reason: "the committed anchors/ps4/nhl20 set is NHL 19 base" }] },
      { game_id: "nhl14", platform: "ps3", title: "NHL 14 (PS3)", status: "available", components: [], gaps: [] },
    ],
  };
}

function openedDto(): ProjectOpenDto {
  return {
    summary: { project_id: 7 } as any,
    project: {
      dir: PROJECT,
      project_id: "abc",
      created: "2026-10-01T00:00:00Z",
      game_id: "nhl21",
      platform: "ps4",
      title: "NHL 21 (PS4)",
      source_kind: "fresh",
      components: [],
      gaps: [],
      destinations: { picked_root: GAME, install_dir: null, game_dir: GAME, save_root: SUGGESTED, roster_save: null, main_db_path: null, meta_xml_path: null, localization_path: null },
      extraction_choice: "pending",
      working_dir: `${PROJECT}\\working`,
      exports_dir: `${PROJECT}\\exports`,
      working_main_db: `${PROJECT}\\working\\db\\nhl_game.db`,
      saves: 0,
      deployments: [],
      staged_files: [],
    } as any,
    discovery: report(),
    warnings: [],
  };
}

function preflight(state: string): ExtractPreflightDto {
  return { root: GAME, state, reason: `state ${state}`, archives: [], members_total: 10, members_present: state === "complete" ? 10 : 0, members_to_write: state === "complete" ? 0 : 10, bytes_to_write: 1024, free_bytes: 1 << 30, collisions: [] };
}

function renderPage() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <SettingsProvider>
        <MemoryRouter>
          <OpenProject />
        </MemoryRouter>
      </SettingsProvider>
    </QueryClientProvider>,
  );
}

const lastCall = (cmd: string) => [...calls].reverse().find((c) => c.cmd === cmd);
const projectInput = () => screen.getByLabelText("Project folder") as HTMLInputElement;
function chooseProjectLocation() {
  fireEvent.change(screen.getByLabelText("Project name"), { target: { value: "nhl21-fresh" } });
}
const gameInput = () => screen.getByLabelText("Game source folder") as HTMLInputElement;
const nextButton = () => screen.getByRole("button", { name: /^Next$|Creating project…/ }) as HTMLButtonElement;

beforeEach(() => {
  vi.mocked(pickDialog).mockReset();
  vi.mocked(confirmDialog).mockReset();
  HTMLDialogElement.prototype.showModal = function () { this.setAttribute("open", ""); };
  HTMLDialogElement.prototype.close = function () { this.removeAttribute("open"); };
  calls.length = 0;
  opened.length = 0;
  localStorage.clear();
  localStorage.setItem(STORAGE_KEY, JSON.stringify({ projectsRoot: "D:\\Studio Projects" }));
  for (const k of Object.keys(handlers)) delete handlers[k];
  handlers.ps4_save_root_suggestion = () => ({ path: SUGGESTED, exists: true });
  handlers.detect_install = () => report();
  handlers.baseline_inventory = () => inventory();
  handlers.project_create_fresh = () => openedDto();
  handlers.project_relink = () => ({ project: openedDto().project, discovery: report(), warnings: [] });
  handlers.project_create_import = () => openedDto();
  handlers.project_import_review = () => ({ ok: true, items: [{ role: "main_db", path: "x", len: 1, sha256: "a", ok: true, detail: "opens" }] });
  handlers.extract_preflight = () => preflight("not_applicable");
  handlers.game_files_check = () => ({ status: "vanilla", reference: "Test disc", region: "USA", matched: 2, loose: 1, containers: 1, findings: [] });
  handlers.extract_running_for = () => null;
  handlers.project_set_extraction = (a: any) => ({ ...openedDto().project, extraction_choice: a.choice });
});
afterEach(() => cleanup());

it("imports a developer package from the Studio start screen and opens it automatically", async () => {
  vi.mocked(pickDialog).mockResolvedValueOnce("D:/Mods/Shared.nhlproject");
  handlers.project_archive_preview = () => ({ name: "Shared", game_id: "nhl21", platform: "ps4" });
  handlers.project_import_archive = () => PROJECT;
  handlers.project_open = () => openedDto();
  renderPage();
  fireEvent.click(screen.getByRole("button", { name: "Import Project Package" }));
  await screen.findByLabelText("Imported project folder");
  expect(vi.mocked(pickDialog)).toHaveBeenCalledWith(expect.objectContaining({ filters: [{ name: "Studio developer project", extensions: ["nhlproject"] }] }));
  fireEvent.click(screen.getByRole("button", { name: "Import and open" }));
  await waitFor(() => expect(opened.length).toBe(1));
  expect(lastCall("project_open")?.args).toEqual({ projectDir: PROJECT });
  expect(calls.some(c => c.cmd === "project_create_fresh")).toBe(false);
  expect(screen.queryByText("Continue to Studio")).toBeNull();
});

it("keeps a failed package-open retryable without importing a second copy", async () => {
  vi.mocked(pickDialog).mockResolvedValueOnce("D:/Mods/Shared.nhlproject");
  handlers.project_archive_preview = () => ({ name: "Shared", game_id: "nhl21", platform: "ps4" });
  handlers.project_import_archive = () => PROJECT;
  handlers.project_open = () => { throw Error("Temporarily unavailable"); };
  renderPage();
  fireEvent.click(screen.getByRole("button", { name: "Import Project Package" }));
  fireEvent.click(await screen.findByRole("button", { name: "Import and open" }));
  await screen.findByText("Project imported, but couldn't open");
  expect(opened).toHaveLength(0);
  handlers.project_open = () => openedDto();
  fireEvent.click(screen.getByRole("button", { name: "Retry opening project" }));
  await waitFor(() => expect(opened.length).toBe(1));
  expect(calls.filter(c => c.cmd === "project_import_archive")).toHaveLength(1);
});

async function toPaths(kind: "Create New Project" | "Import non-studio project" = "Create New Project") {
  renderPage();
  fireEvent.click(screen.getByText(kind));
  fireEvent.click(screen.getByText("NHL 21"));
  chooseProjectLocation();
  await screen.findByRole("heading", { name: "Project Setup" });
  if (kind === "Create New Project") await selectDump();
}

async function selectDump(path = GAME) {
  vi.mocked(pickDialog).mockResolvedValueOnce(path);
  fireEvent.click(screen.getByRole("button", { name: /^(Select Game Source|Change game source)$/ }));
  fireEvent.click(screen.getByRole("button", { name: "Dumped ISO folder" }));
  await screen.findByText("Dumped folder: " + path);
  const dialog = screen.getByRole("dialog");
  fireEvent.click(Array.from(dialog.querySelectorAll("button")).find((b) => b.textContent === "Next")!);
  await screen.findByTestId("game-source-card");
}

describe("ONB-002 onboarding", () => {
  async function setup07() {
    renderPage();
    fireEvent.click(screen.getByText("Create New Project"));
    fireEvent.click(screen.getByText("NHL 07"));
    chooseProjectLocation();
    await screen.findByRole("heading", { name: "Project Setup" });
    expect(screen.queryByText("PS3")).toBeNull();
    expect(screen.queryByLabelText("Save-data location")).toBeNull();
    expect(screen.queryByLabelText("disc image path")).toBeNull();
  }

  async function selectIso() {
    vi.mocked(pickDialog).mockResolvedValueOnce("D:/Images/NHL07.iso");
    fireEvent.click(screen.getByRole("button", { name: "Select Game Source" }));
    fireEvent.click(screen.getByRole("button", { name: "Game ISO" }));
    await screen.findByText("ISO: D:/Images/NHL07.iso");
    expect(pickDialog).toHaveBeenLastCalledWith(expect.objectContaining({ directory: false, filters: [{ name: "Disc image", extensions: ["iso"] }] }));
    fireEvent.click(within(screen.getByRole("dialog")).getByRole("button", { name: "Next" }));
    expect(screen.getByText(/Studio will use this folder as its game source/)).toBeTruthy();
    fireEvent.change(screen.getByLabelText("Extraction destination"), { target: { value: "D:/Games/NHL07" } });
    fireEvent.click(within(screen.getByRole("dialog")).getByRole("button", { name: "Next" }));
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(screen.getByTestId("game-source-card").textContent).toContain("D:/Images/NHL07.iso");
    expect(screen.getByTestId("game-source-card").textContent).toContain("D:/Games/NHL07");
    expect(lastCall("disc_extract_start")).toBeUndefined();
  }

  it.each(["done", "failed", "cancelled"])("NHL07 ISO extraction %s gates fresh anchor creation", async (state) => {
    await setup07();
    await selectIso();
    handlers.disc_extract_start = () => ({ job: 44, state: "running", current: "default.xex", root: "D:/Games/NHL07" });
    handlers.extract_status = () => ({ job: 44, state, current: "default.xex", root: "D:/Games/NHL07", error: state === "done" ? undefined : "Disc copy stopped" });
    fireEvent.click(nextButton());
    await waitFor(() => expect(lastCall("disc_extract_start")?.args).toEqual({ image: "D:/Images/NHL07.iso", root: "D:/Games/NHL07" }));
    expect(lastCall("project_create_fresh")).toBeUndefined();
    if (state === "done") {
      await waitFor(() => expect(lastCall("project_create_fresh")?.args).toMatchObject({ gameId: "nhl07", platform: "xbox", root: "D:/Games/NHL07", savePath: null, rosterSave: null }));
    } else {
      await screen.findByText(/Disc copy stopped/);
      expect(lastCall("project_create_fresh")).toBeUndefined();
      // A terminal error must release Next and retry the same configured source.
      handlers.extract_status = () => ({ job: 44, state: "done", root: "D:/Games/NHL07" });
      fireEvent.click(nextButton());
      await waitFor(() => expect(lastCall("project_create_fresh")?.args).toMatchObject({ root: "D:/Games/NHL07" }));
      expect(lastCall("disc_extract_start")?.args).toEqual({ image: "D:/Images/NHL07.iso", root: "D:/Games/NHL07" });
    }
    expect(lastCall("iso_open")).toBeUndefined();
    expect(lastCall("project_create_import")).toBeUndefined();
  });

  it("lets NHL07 change source and choose a project folder without altering global storage", async () => {
    await setup07();
    await selectDump("D:/Games/First");
    await selectDump("D:/Games/Second");
    vi.mocked(pickDialog).mockResolvedValueOnce("D:/Other/My Project");
    fireEvent.click(screen.getByRole("button", { name: "Browse…" }));
    await waitFor(() => expect(projectInput().value).toBe("D:/Other/My Project"));
    expect(JSON.parse(localStorage.getItem(STORAGE_KEY)!).projectsRoot).toBe("D:\\Studio Projects");
    fireEvent.click(nextButton());
    await waitFor(() => expect(lastCall("project_create_fresh")?.args).toMatchObject({ projectDir: "D:/Other/My Project", root: "D:/Games/Second", savePath: null, rosterSave: null }));
  });

  it("cancelling source changes preserves the original card and does not extract", async () => {
    await setup07();
    await selectDump("D:/Games/First");
    fireEvent.click(screen.getByRole("button", { name: "Change game source" }));
    vi.mocked(pickDialog).mockResolvedValueOnce("D:/Images/Other.iso");
    fireEvent.click(screen.getByRole("button", { name: "Game ISO" }));
    await screen.findByText("ISO: D:/Images/Other.iso");
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    expect(screen.getByTestId("game-source-card").textContent).toContain("D:/Games/First");
    expect(lastCall("disc_extract_start")).toBeUndefined();
  });
  it.each(["Load Project", "Import non-studio project"])("requires first-use storage for %s", (choice) => {
    localStorage.clear();
    renderPage();
    fireEvent.click(screen.getByText(choice));
    expect(screen.getByRole("heading", { name: "Choose your default NHL Project folder" })).toBeTruthy();
    expect((screen.getByRole("button", { name: "Continue" }) as HTMLButtonElement).disabled).toBe(true);
    fireEvent.change(screen.getByLabelText("Projects storage folder"), { target: { value: "D:/Projects" } });
    fireEvent.click(screen.getByRole("button", { name: "Continue" }));
    expect(screen.queryByLabelText("Projects storage folder")).toBeNull();
    if (choice === "Load Project") expect(screen.getByLabelText("Existing project folder")).toBeTruthy();
    else expect(screen.getByRole("heading", { name: "Select Game" })).toBeTruthy();
  });
  it("asks for storage on first use and remembers it without reusing the project name", () => {
    localStorage.clear();
    const page = renderPage();
    expect(screen.queryByLabelText("Project name")).toBeNull();
    fireEvent.click(screen.getByText("Create New Project"));
    fireEvent.change(screen.getByLabelText("Projects storage folder"), { target: { value: "D:\\Studio Projects" } });
    fireEvent.click(screen.getByRole("button", { name: "Continue" }));
    expect(screen.queryByLabelText("Project name")).toBeNull();
    expect(screen.queryByLabelText("Project folder")).toBeNull();
    fireEvent.click(screen.getByText("NHL 21"));
    chooseProjectLocation();
    expect(JSON.parse(localStorage.getItem(STORAGE_KEY)!).projectsRoot).toBe("D:\\Studio Projects");
    page.unmount();
    renderPage();
    fireEvent.click(screen.getByText("Create New Project"));
    expect(screen.queryByLabelText("Projects storage folder")).toBeNull();
    fireEvent.click(screen.getByText("NHL 08"));
    expect(screen.queryByLabelText("Project name")).toBeNull();
    expect(screen.queryByLabelText("Project folder")).toBeNull();
    fireEvent.click(screen.getByText("Xbox 360"));
    expect((screen.getByLabelText("Project name") as HTMLInputElement).value).toBe("");
    fireEvent.change(screen.getByLabelText("Project name"), { target: { value: "NHL08-QA" } });
    expect(projectInput().value).toBe("D:\\Studio Projects\\xbox360\\nhl08\\NHL08-QA");
  });

  it("blocks creation for a name that escapes the storage folder and allows correction", async () => {
    await toPaths();
    fireEvent.change(screen.getByLabelText("Project name"), { target: { value: "../other" } });
    expect(nextButton().disabled).toBe(true);
    expect(projectInput().value).toBe("");
    expect(lastCall("project_create_fresh")).toBeUndefined();
    fireEvent.change(screen.getByLabelText("Project name"), { target: { value: "Valid_roster-QA" } });
    expect(nextButton().disabled).toBe(false);
    fireEvent.click(nextButton());
    await waitFor(() => expect(lastCall("project_create_fresh")?.args.projectDir).toBe("D:\\Studio Projects\\ps4\\nhl21\\Valid_roster-QA"));
  });

  it("retains the PS4 game root when discovery finds a nested database folder", async () => {
    await toPaths("Import non-studio project");
    fireEvent.change(gameInput(), { target: { value: "" } });
    fireEvent.change(screen.getByLabelText("Source assets folder"), { target: { value: GAME } });
    fireEvent.click(screen.getByText("Find database in this folder"));
    await waitFor(() => expect((screen.getByLabelText("Source main database") as HTMLInputElement).value).toBe(report().nhlng_path));
    expect((screen.getByLabelText("Source assets folder") as HTMLInputElement).value).toBe(GAME);
    await waitFor(() => expect(lastCall("project_import_review")?.args.sources.assets_dir).toBe(GAME));
    expect(gameInput().value).toBe("");
  });
  it("finds NHL12 import sources in a relocated disc without linking deployment", async () => {
    const assets = "H:/QA/NHL12 moved disc";
    handlers.detect_install = () => report({ game_id: "nhl12", platform: "xbox", install_dir: assets,
      nhlng_path: `${assets}/db/nhlng.db`, localization_path: `${assets}/fe/loc/nhl_eng_us.db`, saves: [], save_root: null });
    renderPage();
    fireEvent.click(screen.getByText("Import non-studio project"));
    fireEvent.click(screen.getByText("NHL 12"));
    fireEvent.click(await screen.findByText("Xbox 360"));
    chooseProjectLocation();
    await waitFor(() => expect(gameInput()).toBeTruthy());
    fireEvent.change(gameInput(), { target: { value: "" } });
    fireEvent.change(screen.getByLabelText("Source assets folder"), { target: { value: assets } });
    fireEvent.click(screen.getByText("Find database in this folder"));
    await waitFor(() => expect((screen.getByLabelText("Source main database") as HTMLInputElement).value).toBe(`${assets}/db/nhlng.db`));
    expect((screen.getByLabelText("Source dictionary") as HTMLInputElement).value).toBe(`${assets}/fe/loc/nhl_eng_us.db`);
    expect(gameInput().value).toBe("");
    expect(lastCall("detect_install")!.args).toEqual({ gameId: "nhl12", platform: "xbox", root: assets });
  });

  it.each(["metadata", "database", "dictionary"])("NHL07 import displays %s rejection and permits a corrected retry", async (failure) => {
    const detail = `Invalid ${failure} for this project`;
    handlers.project_import_review = ({ sources }) => {
      const invalid = failure === "dictionary" ? !!sources.dictionary : sources.main_db?.includes("invalid");
      return { ok: !invalid, items: [{ role: failure === "dictionary" ? "dictionary" : "main_db", path: sources.main_db,
        len: 1, sha256: "test", ok: !invalid, detail: invalid ? detail : "Files accepted" }] };
    };
    renderPage();
    fireEvent.click(screen.getByText("Import non-studio project"));
    fireEvent.click(screen.getByText("NHL 07"));
    chooseProjectLocation();
    await screen.findByLabelText("Source main database");
    fireEvent.change(screen.getByLabelText("Source main database"), { target: { value: "E:/invalid/NHL07NG.db" } });
    if (failure === "dictionary") fireEvent.change(screen.getByLabelText("Source dictionary"), { target: { value: "E:/invalid.db" } });
    await screen.findByText(detail);
    expect(nextButton().disabled).toBe(true);
    expect(lastCall("project_create_import")).toBeUndefined();
    if (failure === "dictionary") fireEvent.change(screen.getByLabelText("Source dictionary"), { target: { value: "" } });
    else fireEvent.change(screen.getByLabelText("Source main database"), { target: { value: "E:/valid/NHL07NG.db" } });
    await screen.findByText("Files accepted");
    await waitFor(() => expect(nextButton().disabled).toBe(false));
    fireEvent.click(nextButton());
    await waitFor(() => expect(lastCall("project_create_import")).toBeTruthy());
    expect(lastCall("project_create_import")!.args).toMatchObject({ gameId: "nhl07", platform: "xbox", savePath: null, rosterSave: null });
  });

  it("opens NHL07 with an installation and main database but no save-data folder", async () => {
    handlers.detect_install = () => report({ game_id: "nhl07", platform: "xbox", saves: [], save_root: null });
    renderPage();
    fireEvent.click(screen.getByText("Import non-studio project"));
    fireEvent.click(screen.getByText("NHL 07"));
    chooseProjectLocation();
    await waitFor(() => expect(gameInput()).toBeTruthy());
    fireEvent.change(gameInput(), { target: { value: "E:/copied NHL07" } });
    expect(screen.queryByLabelText("Save-data location")).toBeNull();
    await waitFor(() => expect(lastCall("detect_install")?.args.savePath).toBe(null));
    fireEvent.change(screen.getByLabelText("Source main database"), { target: { value: "E:/copied NHL07/db/NHL07NG.db" } });
    await waitFor(() => expect(nextButton().disabled).toBe(false));
    fireEvent.click(nextButton());
    await waitFor(() => expect(lastCall("project_create_import")).toBeTruthy());
    expect(lastCall("project_create_import")!.args).toMatchObject({ gameId: "nhl07", platform: "xbox", root: "E:/copied NHL07", savePath: null });
  });

  it("offers extraction for imported packed assets without a deployment installation", async () => {
    const assets = "E:/Relocated disc dump";
    handlers.project_create_import = () => ({ ...openedDto(), discovery: report({ game_dir: null, install_dir: assets, picked_root: "", saves: [] }) });
    handlers.extract_preflight = () => preflight("needed");
    await toPaths("Import non-studio project");
    fireEvent.change(screen.getByLabelText("Source main database"), { target: { value: "E:/mine/nhl_game.db" } });
    fireEvent.change(screen.getByLabelText("Source assets folder"), { target: { value: assets } });
    await screen.findByText("opens");
    await waitFor(() => expect(nextButton().disabled).toBe(false));
    fireEvent.click(nextButton());
    await screen.findByRole("heading", { name: "Extract game assets now?" });
    expect(lastCall("project_create_import")!.args.root).toBe("");
    expect(lastCall("extract_preflight")!.args).toEqual({ root: assets, platform: "ps4" });
    expect(opened).toHaveLength(0);
  });

  it("fresh setup hides import/save controls and creates from anchors with only the selected source destination", async () => {
    await toPaths();
    expect(projectInput().value).toBe(PROJECT);
    expect(screen.queryByLabelText("Game source folder")).toBeNull();
    expect(screen.queryByLabelText("Save-data location")).toBeNull();
    expect(screen.queryByLabelText("Source main database")).toBeNull();
    expect(screen.getByTestId("game-source-card").textContent).toContain(GAME);
    fireEvent.click(nextButton());
    await waitFor(() => expect(lastCall("project_create_fresh")).toBeTruthy());
    expect(lastCall("project_create_fresh")!.args).toEqual({ projectDir: PROJECT, gameId: "nhl21", platform: "ps4", root: GAME, savePath: null, rosterSave: null });
    expect(lastCall("project_create_import")).toBeUndefined();
    await waitFor(() => expect(opened.length).toBe(1));
  });

  it("shows the extraction choice only when preflight says something needs extracting, and Skip opens the project without extracting", async () => {
    handlers.extract_preflight = () => preflight("needed");
    handlers.detect_install = () => report({ saves: [roster("1")] });
    await toPaths();
    await waitFor(() => expect(nextButton().disabled).toBe(false));
    fireEvent.click(nextButton());
    await screen.findByRole("heading", { name: "Extract game assets now?" });
    expect(screen.getByText(/You can do this later in Settings → Tools/)).toBeTruthy();
    expect(opened.length).toBe(0);
    await waitFor(() => expect((screen.getByRole("button", { name: "Skip for now" }) as HTMLButtonElement).disabled).toBe(false));
    fireEvent.click(screen.getByRole("button", { name: "Skip for now" }));
    await waitFor(() => expect(opened.length).toBe(1));
    expect(lastCall("project_set_extraction")!.args).toEqual({ projectId: 7, choice: "skipped" });
    expect(calls.some((c) => c.cmd === "extract_start")).toBe(false);
  });

  it("Extract now starts the job, shows progress, and continues after it finishes", async () => {
    handlers.extract_preflight = () => preflight("needed");
    handlers.detect_install = () => report({ saves: [roster("1")] });
    let polls = 0;
    const job = (state: string) => ({ job: 3, root: GAME, state, current: `${GAME}\\databigs\\game_90.big`, archives_done: 1, archives_total: 2, files_written: 5, files_retained: 1, files_conflicting: 1, files_blocked: 0, files_unsupported: 0, files_unsafe: 0, archives_failed: 0, bytes_written: 4096, problems: ["x exists with different bytes"], error: null });
    handlers.extract_start = () => job("running");
    handlers.extract_status = () => job(++polls >= 2 ? "done" : "running");
    await toPaths();
    await waitFor(() => expect(nextButton().disabled).toBe(false));
    fireEvent.click(nextButton());
    await screen.findByRole("heading", { name: "Extract game assets now?" });
    await waitFor(() => expect((screen.getByRole("button", { name: "Extract now" }) as HTMLButtonElement).disabled).toBe(false));
    fireEvent.click(screen.getByRole("button", { name: "Extract now" }));
    await waitFor(() => expect(lastCall("extract_start")).toBeTruthy());
    expect(lastCall("extract_start")!.args).toEqual({ root: GAME, platform: "ps4", destination: GAME });
    await screen.findByText(/Extraction finished/, {}, { timeout: 3000 });
    expect(screen.getByText("Extraction finished. Some files were skipped.")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Continue to Studio" }));
    await waitFor(() => expect(opened.length).toBe(1));
    expect(lastCall("project_set_extraction")!.args).toEqual({ projectId: 7, choice: "skipped" });
  });

  it.each(["Skip for now", "Continue to Studio"])("requires acknowledgement before %s for a differing installation", async action => {
    handlers.extract_preflight = () => preflight("partial");
    handlers.game_files_check = () => ({ status: "different", reference: "NHL 07 USA", region: "USA", matched: 1, loose: 1, containers: 1, findings: [{ path: "default.xex", issue: "SHA-256 differs from reference" }] });
    await toPaths();
    fireEvent.click(nextButton());
    const checkbox = await screen.findByRole("checkbox", { name: "Continue with modified files" });
    for (const name of ["Extract now", "Skip for now", "Continue to Studio"]) {
      expect((screen.getByRole("button", { name }) as HTMLButtonElement).disabled).toBe(true);
    }
    fireEvent.click(checkbox);
    await waitFor(() => expect((screen.getByRole("button", { name: action }) as HTMLButtonElement).disabled).toBe(false));
    fireEvent.click(screen.getByRole("button", { name: action }));
    await waitFor(() => expect(opened.length).toBe(1));
    expect(lastCall("extract_start")).toBeUndefined();
  });

  it("Import non-studio project asks for the source files, reviews them, and never treats them as destinations", async () => {
    handlers.detect_install = () => report({ saves: [roster("1")] });
    await toPaths("Import non-studio project");
    expect(screen.getByTestId("import-sources")).toBeTruthy();
    fireEvent.change(gameInput(), { target: { value: GAME } });
    await waitFor(() => expect(lastCall("detect_install")).toBeTruthy());
    expect(nextButton().disabled).toBe(true);
    fireEvent.change(screen.getByLabelText("Source main database"), { target: { value: "E:\\mine\\nhl_game.db" } });
    await waitFor(() => expect(lastCall("project_import_review")).toBeTruthy());
    // Optional sources are omitted (undefined), never sent as destinations.
    expect(lastCall("project_import_review")!.args).toEqual({ gameId: "nhl21", platform: "ps4", sources: { main_db: "E:\\mine\\nhl_game.db" } });
    await screen.findByText("opens");
    await waitFor(() => expect(nextButton().disabled).toBe(false));
    fireEvent.click(nextButton());
    await waitFor(() => expect(lastCall("project_create_import")).toBeTruthy());
    const args = lastCall("project_create_import")!.args;
    expect(args.sources.main_db).toBe("E:\\mine\\nhl_game.db");
    expect(args.root).toBe(GAME);
    expect(args.projectDir).toBe(PROJECT);
    // A failing review disables Next and names the problem.
    cleanup();
    calls.length = 0;
    handlers.project_import_review = () => ({ ok: false, items: [{ role: "roster_save", path: "x", len: 1, sha256: "a", ok: false, detail: "not a save container Studio can read" }] });
    await toPaths("Import non-studio project");
    fireEvent.change(gameInput(), { target: { value: GAME } });
    fireEvent.change(screen.getByLabelText("Source main database"), { target: { value: "E:\\mine\\nhl_game.db" } });
    await screen.findByText("not a save container Studio can read");
    expect(nextButton().disabled).toBe(true);
  });

  it("refuses Create New Project for a title whose bundled baseline is unavailable, naming the gap", async () => {
    renderPage();
    fireEvent.click(screen.getByText("Create New Project"));
    fireEvent.click(screen.getByText("NHL 20"));
    chooseProjectLocation();
    await screen.findByTestId("baseline-note");
    // The baseline note and the Next reason both say so (ONB-003: the reason no longer waits for an installation folder).
    expect(screen.getAllByText(/not available/).length).toBeGreaterThan(0);
    expect(screen.getByText(/NHL 19 base/)).toBeTruthy();

    expect(nextButton().disabled).toBe(true);
    expect(screen.getByText(/Start fresh is not available for this title/)).toBeTruthy();
  });

  it("remembered save preferences never attach a save or block a database import", async () => {
    localStorage.setItem(STORAGE_KEY, JSON.stringify({ projectsRoot: "D:\\Studio Projects", defaultGameId: "nhl21", defaultPlatform: "ps4", paths: { "nhl21:ps4": { root: GAME, savePath: SUGGESTED, saveFile: roster("9").path, projectDir: PROJECT } } }));
    handlers.detect_install = () => report({ saves: [roster("1")] });
    renderPage();
    // First screen, with the remembered target named; nothing was called yet except the inventory.
    expect(screen.getByText("Create New Project")).toBeTruthy();
    // Only read-only lookups ran (the inventory and the PS4 save-folder suggestion): no scan, no open.
    expect(calls.filter((c) => c.cmd !== "baseline_inventory" && c.cmd !== "ps4_save_root_suggestion").length).toBe(0);
    expect(opened.length).toBe(0);
    fireEvent.click(screen.getByText("Import non-studio project"));
    fireEvent.click(screen.getByText("NHL 21"));
    chooseProjectLocation();
    await waitFor(() => expect(gameInput().value).toBe(GAME));
    expect(projectInput().value).toBe(PROJECT);
    fireEvent.change(screen.getByLabelText("Source main database"), { target: { value: "D:/source/nhl_game.db" } });
    await waitFor(() => expect(lastCall("project_import_review")).toBeTruthy());
    await waitFor(() => expect(lastCall("detect_install")).toBeTruthy());
    expect(screen.queryByLabelText("Save-data location")).toBeNull();
    expect(screen.queryAllByRole("radio")).toHaveLength(0);
    await waitFor(() => expect(nextButton().disabled).toBe(false));
    fireEvent.click(nextButton());
    await waitFor(() => expect(lastCall("project_create_import")).toBeTruthy());
    expect(lastCall("project_create_import")!.args.rosterSave).toBeNull();
    expect(lastCall("project_create_import")!.args.savePath).toBeNull();
    expect(calls.some((c) => c.cmd === "open_workspace")).toBe(false);
  });

  it("ONB-003: Open existing project opens the folder, shows the resume notes and lands on the last view", async () => {
    handlers.project_open = (a: any) => ({ ...openedDto(), project: { ...openedDto().project, dir: a.projectDir, saves: 2, last_saved: "2026-10-01T10:00:00Z" }, warnings: ["the game installation D:\Games is not reachable; database and asset work continues"], last_view: "/app/database/players/4" });
    renderPage();
    fireEvent.click(screen.getByText("Load Project"));
    fireEvent.change(screen.getByLabelText("Existing project folder"), { target: { value: "D:\Studio Projects\old" } });
    fireEvent.click(screen.getByRole("button", { name: "Open" }));
    await waitFor(() => expect(lastCall("project_open")).toBeTruthy());
    expect(lastCall("project_open")!.args).toEqual({ projectDir: "D:\Studio Projects\old" });
    await screen.findByRole("heading", { name: "Resume project" });
    expect(screen.getByTestId("resume-warnings").textContent).toContain("not reachable");
    expect(opened.length).toBe(0);
    // No creation, no import, no deployment, no extraction happened on open.
    expect(calls.some((c) => ["project_create_fresh", "project_create_import", "extract_start", "project_deploy_apply"].includes(c.cmd))).toBe(false);
    fireEvent.click(screen.getByRole("button", { name: "Continue where I left off" }));
    await waitFor(() => expect(opened.length).toBe(1));
    const stored = JSON.parse(localStorage.getItem(STORAGE_KEY)!);
    expect(stored.recentProjects).toEqual(["D:\Studio Projects\old"]);
  });

  it("imports a game database without installation or save inputs", async () => {
    handlers.project_create_import = () => ({ ...openedDto(), discovery: report({ game_dir: null, install_dir: null, nhlng_path: null, meta_xml_path: null, saves: [], picked_root: "" }) });
    await toPaths("Import non-studio project");
    expect(nextButton().disabled).toBe(true);
    expect(screen.queryByLabelText("Source roster save")).toBeNull();
    expect(screen.queryByLabelText("Save-data location")).toBeNull();
    fireEvent.change(screen.getByLabelText("Source main database"), { target: { value: "E:/mine/nhl_game.db" } });
    await waitFor(() => expect(nextButton().disabled).toBe(false));
    fireEvent.click(nextButton());
    await waitFor(() => expect(lastCall("project_create_import")).toBeTruthy());
    const args = lastCall("project_create_import")!.args;
    expect(args.root).toBe("");
    expect(args.sources).toEqual({ main_db: "E:/mine/nhl_game.db" });
    expect(args.rosterSave).toBeNull();
    expect(args.savePath).toBeNull();
    await waitFor(() => expect(opened.length).toBe(1));
    expect(calls.some((c) => c.cmd === "extract_preflight")).toBe(false);
  });

  it("the game and console screens are unchanged: NHL 15 asks for the console before the paths", async () => {
    handlers.baseline_inventory = () => inventory("available");
    renderPage();
    fireEvent.click(screen.getByText("Create New Project"));
    fireEvent.click(screen.getByText("NHL 15"));
    await screen.findByText("Select Console");
    expect(screen.queryByLabelText("Project name")).toBeNull();
    expect(screen.queryByLabelText("Project folder")).toBeNull();
    expect(screen.getByText("PS3")).toBeTruthy();
    expect(screen.getByText("Xbox 360")).toBeTruthy();
    expect(screen.getByText("PS4")).toBeTruthy();
    fireEvent.click(screen.getByText("PS3"));
    await screen.findByRole("heading", { name: "Project Setup" });
    chooseProjectLocation();
    expect(screen.queryByLabelText("Save-data location")).toBeNull();
    expect(screen.getByRole("button", { name: "Select Game Source" })).toBeTruthy();
  });
});


it.each(["cancel", "delete", "failure", "forget"])("handles recent-project %s without losing the wrong entry", async mode => {
  localStorage.setItem(STORAGE_KEY, JSON.stringify({ projectsRoot: "D:/Projects", recentProjects: ["D:/Projects/QA"] }));
  vi.mocked(confirmDialog).mockResolvedValue(mode !== "cancel");
  handlers.project_delete = () => { if (mode === "failure") throw new Error("Project is open"); return null; };
  renderPage();
  fireEvent.click(screen.getByRole("button", { name: "Load Project" }));
  fireEvent.click(await screen.findByRole("button", { name: mode === "forget" ? "Forget" : "Delete" }));
  if (mode === "forget" || mode === "delete") {
    await waitFor(() => expect(JSON.parse(localStorage.getItem(STORAGE_KEY)!).recentProjects).toEqual([]));
  } else if (mode === "failure") {
    await screen.findByText(/Project is open/);
    expect(JSON.parse(localStorage.getItem(STORAGE_KEY)!).recentProjects).toEqual(["D:/Projects/QA"]);
  } else {
    await waitFor(() => expect(confirmDialog).toHaveBeenCalled());
    expect(JSON.parse(localStorage.getItem(STORAGE_KEY)!).recentProjects).toEqual(["D:/Projects/QA"]);
  }
  if (mode === "delete" || mode === "failure") expect(lastCall("project_delete")?.args).toEqual({ projectDir: "D:/Projects/QA" });
  else expect(lastCall("project_delete")).toBeUndefined();
});

it.each(["done", "failed", "cancelled"])("PS3 decrypted ISO extraction %s gates project creation and supports retry", async state => {
  renderPage();
  fireEvent.click(screen.getByText("Create New Project"));
  fireEvent.click(screen.getByText("NHL 08"));
  fireEvent.click(screen.getByText("PS3"));
  chooseProjectLocation();
  const image = "D:/Images/NHL 08 (USA).dec.iso", root = "D:/Games/NHL 08 (USA)";
  vi.mocked(pickDialog).mockResolvedValueOnce(image);
  fireEvent.click(screen.getByRole("button", { name: "Select Game Source" }));
  const iso = screen.getByRole("button", { name: "Game ISO" }) as HTMLButtonElement;
  expect(iso.disabled).toBe(false);
  expect(screen.getByText(/Use a decrypted PS3 ISO/)).toBeTruthy();
  fireEvent.click(iso);
  await screen.findByText("ISO: " + image);
  fireEvent.click(within(screen.getByRole("dialog")).getByRole("button", { name: "Next" }));
  vi.mocked(pickDialog).mockResolvedValueOnce("D:/Games");
  fireEvent.click(screen.getByRole("button", { name: "Browse extraction location…" }));
  await waitFor(() => expect((screen.getByLabelText("Extraction destination") as HTMLInputElement).value).toBe(root));
  fireEvent.click(within(screen.getByRole("dialog")).getByRole("button", { name: "Next" }));
  handlers.disc_extract_start = () => ({ job: 45, state: "running", root });
  handlers.extract_status = () => ({ job: 45, state, root, error: state === "done" ? undefined : "Disc copy stopped" });
  handlers.detect_install = () => report({ game_id: "nhl08", platform: "ps3", install_dir: root + "/PS3_GAME/USRDIR", game_dir: null });
  fireEvent.click(nextButton());
  await waitFor(() => expect(lastCall("disc_extract_start")?.args).toEqual({ image, root }));
  expect(lastCall("project_create_fresh")).toBeUndefined();
  if (state !== "done") {
    await screen.findByText(/Disc copy stopped/);
    expect(lastCall("project_create_fresh")).toBeUndefined();
    handlers.extract_status = () => ({ job: 45, state: "done", root });
    fireEvent.click(nextButton());
  }
  await waitFor(() => expect(lastCall("project_create_fresh")?.args).toMatchObject({ gameId: "nhl08", platform: "ps3", root, savePath: null, rosterSave: null }));
});
