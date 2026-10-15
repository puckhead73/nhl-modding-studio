/** Derive one child folder; never silently sanitize a name into a different project. */
export function projectLocation(root: string, name: string): { path: string; error: string | null } {
  const parent = root.trim();
  if (!parent) return { path: "", error: "Choose a projects storage folder." };
  if (!/^(?:[a-z]:[\\/]|\\\\[^\\/]+[\\/][^\\/]+(?:[\\/]|$)|\/(?!\/))/i.test(parent)) {
    return { path: "", error: "Use an absolute projects storage path." };
  }
  if (!name) return { path: "", error: "Enter a project name." };
  // Report only present issues, retaining every applicable correction for mixed errors.
  const issues: string[] = [];
  if (/^\s/.test(name)) issues.push("Remove leading spaces.");
  if (/\s$/.test(name)) issues.push("Remove trailing spaces.");
  if (name.startsWith(".")) issues.push("Remove leading periods.");
  if (name.endsWith(".")) issues.push("Remove trailing periods.");
  const reserved = [...new Set(name.match(/[<>:"/\\|?*]/g) ?? [])];
  if (reserved.length) issues.push(`Remove reserved characters: ${reserved.map(c => `'${c}'`).join(", ")}.`);
  if (/[\x00-\x1f]/.test(name)) issues.push("Remove control characters.");
  if (/^(con|prn|aux|nul|com[1-9¹²³]|lpt[1-9¹²³])(?:\.|$)/i.test(name)) issues.push("Choose a name that is not a reserved Windows device name.");
  if (name.length > 255) issues.push("Shorten the project name to 255 characters or fewer.");
  if (issues.length) return { path: "", error: issues.join(" ") };
  const separator = parent.includes("\\") ? "\\" : "/";
  return { path: parent.replace(/[\\/]+$/, "") + separator + name, error: null };
}

export const platformFolder = (platform: string) => platform === "xbox" ? "xbox360" : platform;

/** The stable catalog ids keep folders independent of display labels and localization. */
export function organizedProjectLocation(root: string, name: string, platform?: string | null, game?: string | null) {
  const named = projectLocation(root, name);
  if (named.error) return named;
  if (!platform || !game) return { path: "", error: "Select a game and platform." };
  const p = projectLocation(root, platformFolder(platform));
  if (p.error) return p;
  const g = projectLocation(p.path, game);
  return g.error ? g : projectLocation(g.path, name);
}

/** A multi-platform release lists its declared platforms in a stable order. */
export function releaseBuildLocation(root: string, name: string, platforms: string[], game: string) {
  const dir = projectLocation(root, "Builds");
  if (dir.error) return dir;
  const validName = projectLocation(root, name);
  if (validName.error) return validName;
  if (!platforms.length || !game) return { path: "", error: "Select the build's game and platforms." };
  return projectLocation(dir.path, `${name}_${[...new Set(platforms.map(platformFolder))].sort().join("-")}_${game}.nhlmod`);
}
