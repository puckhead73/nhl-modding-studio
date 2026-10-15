import type { InstallReportDto } from "../bindings/InstallReportDto";

/** PS4 discovery separates the database directory from the game asset root.
 * Own-file projects without an installation expose their selected assets folder as install_dir. */
export function assetRoot(install: InstallReportDto | null): string | null {
  if (!install) return null;
  return (install.platform === "ps4" ? install.game_dir ?? install.install_dir : install.install_dir) ?? null;
}
