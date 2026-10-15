// @vitest-environment jsdom
//
// Phase 4 (21 Sep 2026) — the equipment files card: rows with files, a "no files" warning for an
// id the title does not ship, a "no folder" neutral state; args carry the asset roots; an error
// shows as text; an empty listing renders nothing. jsdom only.
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const calls: Array<{ cmd: string; args: any }> = [];
let response: any = [];
let failWith: string | null = null;
vi.mock("../lib/client", () => ({
  call: (cmd: string, args: any) => {
    calls.push({ cmd, args });
    if (failWith) return Promise.reject(new Error(failWith));
    return Promise.resolve(response);
  },
}));

import { EquipmentFilesCard } from "./EquipmentFilesCard";

const roots = { install_dir: "X:/NHL Legacy", platform: "xbox", rendering_override: null, fe_override: null } as any;

function mount() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <EquipmentFilesCard projectId={7} playerId={42} roots={roots} />
    </QueryClientProvider>,
  );
}

describe("EquipmentFilesCard", () => {
  beforeEach(() => {
    calls.length = 0;
    response = [];
    failWith = null;
  });
  afterEach(cleanup);

  it("lists the slots with their file state", async () => {
    response = [
      { slot: "gloves", label: "Gloves", field_ref: "xmja.NmUR", folder: "glove", id: 27, model_label: "Bauer Supreme", files: ["glove/glove_27_0_dm.rx2"], file_count: 6, catalogue_ids: 56, catalogue_min: 27, catalogue_max: 83, note: "6 files (catalogue: 56 ids, 27–83)", set_contract: "measured", set_rule: "the family's majority component set (R52)", component_groups: [{ group: "in-game", status: "complete", present: ["glove_27_am"], missing: [] }, { group: "front-end", status: "incomplete", present: ["feglove_27_0_tm"], missing: ["feglove_27_0_dm"] }] },
      { slot: "helmet", label: "Helmet", field_ref: "xmja.YUZp", folder: "helmet", id: 99, model_label: null, files: [], file_count: 0, catalogue_ids: 21, catalogue_min: 1, catalogue_max: 21, note: "no files for id 99 in `helmet` — the game has no art for this value on this title (the catalogue holds 21 ids, 1–21)", set_contract: "single_container", set_rule: "one `texlib_<id>` container per id", component_groups: [] },
      { slot: "skates", label: "Skates", field_ref: "xmja.AOOe", folder: "skate", id: 3, model_label: null, files: [], file_count: 0, catalogue_ids: 0, catalogue_min: null, catalogue_max: null, note: "no `skate` folder under this install's rendering tree (archive-only title, or no install tree)", set_contract: "not_measured", set_rule: "not measured", component_groups: [] },
    ];
    mount();
    await waitFor(() => expect(screen.getByTestId("equipment-files")).toBeTruthy());
    expect(calls).toEqual([{ cmd: "player_equipment_files", args: { projectId: 7, playerId: 42, roots } }]);
    expect(screen.getByTestId("equipment-files-gloves").textContent).toContain("6 files");
    expect(screen.getByTestId("equipment-files-gloves").textContent).toContain("Bauer Supreme");
    expect(screen.getByTestId("equipment-files-helmet").textContent).toContain("no files");
    expect(screen.getByTestId("equipment-files-helmet").textContent).toContain("no art for this value");
    expect(screen.getByTestId("equipment-files-skates").textContent).toContain("no folder");
    // Phase 4 (22 Sep 2026): the component set per group — complete, or what the id lacks.
    expect(screen.getByTestId("equipment-set-gloves-in-game").textContent).toContain("in-game set complete");
    expect(screen.getByTestId("equipment-set-gloves-front-end").textContent).toContain("front-end set missing feglove_27_0_dm");
    expect(screen.queryByTestId("equipment-set-helmet-in-game")).toBeNull();
    expect(screen.queryByTestId("equipment-set-skates-unmeasured")).toBeNull();
  });

  it("says when the title's set is not measured, and only for ids that have files", async () => {
    response = [
      { slot: "gloves", label: "Gloves", field_ref: "xmja.NmUR", folder: "glove", id: 3, model_label: null, files: ["glove/texlib_3.rx2"], file_count: 1, catalogue_ids: 19, catalogue_min: 0, catalogue_max: 30, note: "1 file (catalogue: 19 ids, 0–30)", set_contract: "not_measured", set_rule: "the equipment component sets of `nhl09` are not measured", component_groups: [] },
      { slot: "helmet", label: "Helmet", field_ref: "xmja.YUZp", folder: "helmet", id: 9, model_label: null, files: [], file_count: 0, catalogue_ids: 9, catalogue_min: 1, catalogue_max: 9, note: "no files for id 9", set_contract: "not_measured", set_rule: "not measured", component_groups: [] },
    ];
    mount();
    await waitFor(() => expect(screen.getByTestId("equipment-files")).toBeTruthy());
    expect(screen.getByTestId("equipment-set-gloves-unmeasured").textContent).toContain("set not measured on this title");
    expect(screen.queryByTestId("equipment-set-helmet-unmeasured")).toBeNull();
  });

  it("renders nothing for an empty listing and the error as text", async () => {
    const r = mount();
    await new Promise((res) => setTimeout(res, 20));
    expect(r.container.textContent).toBe("");
    cleanup();
    failWith = "no player with id 42";
    mount();
    await waitFor(() => expect(screen.getByTestId("equipment-files-error").textContent).toContain("no player"));
  });
});
