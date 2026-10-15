// @vitest-environment jsdom
//
// LEAGUE-PARITY-COMPLETION-R1 F3 — the shared project-summary publication boundary: `update`
// adopts a summary only for the project that is open now, so a late response for a closed or
// replaced project can never pair the new project's paths and install with the old summary.
import { act, cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import type { ProjectSummaryDto } from "../bindings/ProjectSummaryDto";
import { ProjectProvider, useProject } from "./project";

function summary(project_id: number, workspace_revision: string): ProjectSummaryDto {
  return {
    project_id,
    game_id: "nhl21",
    platform: "ps4",
    role: "nhlng",
    source: "raw_db",
    endian: "Little",
    table_count: 1,
    db_size: 1,
    header_crc_ok: true,
    size_ok: true,
    dirty: false,
    undo_depth: 0,
    redo_depth: 0,
    tables: [],
    roster_attached: false,
    roster_aligned: false,
    display_roster: false,
    capabilities: {} as ProjectSummaryDto["capabilities"],
    surfaces: {} as ProjectSummaryDto["surfaces"],
    write_experimental: false,
    workspace_revision,
  };
}

let ctx: ReturnType<typeof useProject>;
function Probe() {
  ctx = useProject();
  return null;
}

afterEach(() => cleanup());

describe("ProjectProvider.update", () => {
  it("adopts a summary for the open project and drops one for any other project", () => {
    render(
      <ProjectProvider>
        <Probe />
      </ProjectProvider>,
    );
    act(() => ctx.open(summary(1, "a1"), "C:/one/nhlng.db"));
    act(() => ctx.update(summary(1, "a2")));
    expect(ctx.project?.workspace_revision).toBe("a2");
    expect(ctx.dbPath).toBe("C:/one/nhlng.db");

    // Project 1 closes, project 2 opens; a late summary for project 1 arrives.
    act(() => ctx.close());
    act(() => ctx.open(summary(2, "b1"), "C:/two/nhlng.db"));
    act(() => ctx.update(summary(1, "a3-late")));
    expect(ctx.project?.project_id).toBe(2);
    expect(ctx.project?.workspace_revision).toBe("b1");
    expect(ctx.dbPath).toBe("C:/two/nhlng.db");

    // Project 2's own summary is adopted; a summary while nothing is open is a no-op.
    act(() => ctx.update(summary(2, "b2")));
    expect(ctx.project?.workspace_revision).toBe("b2");
    act(() => ctx.close());
    act(() => ctx.update(summary(2, "b3")));
    expect(ctx.project).toBeNull();
  });
});

// Relinking changes the resource binding immediately, without reopening or dropping staged work.
it("publishes relink discovery only into the same open session", () => {
  render(<ProjectProvider><Probe /></ProjectProvider>);
  const info = { dir: "D:/OwnedProject" } as import("../bindings/ProjectInfoDto").ProjectInfoDto;
  const oldInstall = { platform: "ps4", game_dir: null, install_dir: null } as import("../bindings/InstallReportDto").InstallReportDto;
  const linked = { ...oldInstall, game_dir: "D:/NHL21", install_dir: "D:/NHL21/db" };
  act(() => ctx.open(summary(1, "first"), "working.db", oldInstall, null, info));
  act(() => ctx.updateInfo(info, linked, 1));
  expect(ctx.install?.game_dir).toBe("D:/NHL21");
  expect(ctx.dbPath).toBe("working.db");
  act(() => ctx.open(summary(2, "reopened"), "working.db", oldInstall, null, info));
  act(() => ctx.updateInfo(info, linked, 1));
  expect(ctx.install?.game_dir).toBeNull();
});
