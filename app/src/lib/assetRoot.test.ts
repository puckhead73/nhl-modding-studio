import { describe, expect, it } from "vitest";
import { assetRoot } from "./assetRoot";
import type { InstallReportDto } from "../bindings/InstallReportDto";
const report = (platform: string, game_dir: string | null, install_dir: string | null) => ({ platform, game_dir, install_dir } as InstallReportDto);
describe("asset roots from discovery", () => {
  it("uses the PS4 game folder even when its database was found in a deeply extracted subtree", () => {
    expect(assetRoot(report("ps4", "D:/NHL21", "D:/NHL21/databigs/Extracted/nhl/data/common/gamemodes/database/t3db"))).toBe("D:/NHL21");
  });
  it("keeps the explicit assets-only input when no PS4 installation is linked", () => {
    expect(assetRoot(report("ps4", null, "D:/OwnedAssets"))).toBe("D:/OwnedAssets");
  });
  it("keeps native Xbox, PS3 and Recompiled roots for their platform resolvers", () => {
    for (const platform of ["xbox", "ps3", "recomp"]) expect(assetRoot(report(platform, null, "D:/OwnedGame"))).toBe("D:/OwnedGame");
  });
  it("does not invent an asset directory for database-only or roster-only input", () => {
    expect(assetRoot(report("ps4", null, null))).toBeNull();
    expect(assetRoot(null)).toBeNull();
  });
});
