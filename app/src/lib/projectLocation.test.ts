import { describe, expect, it } from "vitest";
import { projectLocation, organizedProjectLocation, releaseBuildLocation } from "./projectLocation";

describe("named project folders", () => {
  it.each([
    [" roster", "Remove leading spaces."],
    ["roster ", "Remove trailing spaces."],
    [".roster", "Remove leading periods."],
    ["roster.", "Remove trailing periods."],
    ["roster/QA", "Remove reserved characters: '/'."],
    ["roster\\QA", "Remove reserved characters: '\\'."],
    [" roster/QA.", "Remove leading spaces. Remove trailing periods. Remove reserved characters: '/'."],
    ["CON", "Choose a name that is not a reserved Windows device name."],
    ["a".repeat(256), "Shorten the project name to 255 characters or fewer."],
  ])("reports only the issues in %j", (name, error) => {
    expect(projectLocation("D:/Projects", name)).toEqual({ path: "", error });
  });
  it.each(["NHL-07", "NHL_07", "NHL_07-QA", "NHL 07 - QA", "-NHL_07-"])("preserves hyphens and underscores in %s", (name) => {
    expect(organizedProjectLocation("E:\\NHL_Projects-QA", name, "xbox", "nhl07")).toEqual({
      path: `E:\\NHL_Projects-QA\\xbox360\\nhl07\\${name}`, error: null,
    });
  });
  it("organizes projects by platform and game and keeps releases in Builds", () => {
    expect(organizedProjectLocation("D:/Projects", "Legends", "xbox", "nhl14").path).toBe("D:/Projects/xbox360/nhl14/Legends");
    expect(organizedProjectLocation("D:/Projects", "Legends", "ps3", "nhl14").path).toBe("D:/Projects/ps3/nhl14/Legends");
    expect(organizedProjectLocation("D:/Projects", "Legends", null, "nhl14").error).not.toBeNull();
    expect(releaseBuildLocation("D:/Projects", "Legends", ["xbox"], "nhl14").path).toBe("D:/Projects/Builds/Legends_xbox360_nhl14.nhlmod");
    expect(releaseBuildLocation("D:/Projects", "Legends", ["xbox", "ps3", "xbox"], "nhl14").path).toBe("D:/Projects/Builds/Legends_ps3-xbox360_nhl14.nhlmod");
    expect(releaseBuildLocation("D:/Projects", "../escape", ["xbox"], "nhl14").error).not.toBeNull();
  });
  it.each([
    ["D:\\Studio Projects\\", "Legends", "D:\\Studio Projects\\Legends"],
    ["D:/Studio Projects/", "NHL 14", "D:/Studio Projects/NHL 14"],
    ["D:\\", "Legends", "D:\\Legends"],
    ["\\\\server\\share\\", "Legends", "\\\\server\\share\\Legends"],
    ["/projects/", "Legends", "/projects/Legends"],
  ])("creates one child of %s", (root, name, path) => {
    expect(projectLocation(root, name)).toEqual({ path, error: null });
  });

  it.each(["", ".", "..", "../escape", "a\\b", "C:other", "CON", "nul.txt", "LPT1", "COM¹", "trailing.", " space", "space ", "a?b", "a\u0000b", "a".repeat(256)])("rejects invalid name %j", (name) => {
    expect(projectLocation("D:\\Projects", name).error).not.toBeNull();
    expect(projectLocation("D:\\Projects", name).path).toBe("");
  });

  it.each(["", "projects", "D:projects", "\\projects"])("rejects missing or relative root %j", (root) => {
    expect(projectLocation(root, "Legends").error).not.toBeNull();
  });
});
