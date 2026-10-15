import { createContext, useContext } from "react";

/// Lets a detail view collapse the surrounding EntityList master list (e.g. the Uniforms tab hands
/// the full width to the kit view). A no-op outside an EntityList.
export const SidebarCollapseContext = createContext<(collapsed: boolean) => void>(() => {});

export function useSidebarCollapse() {
  return useContext(SidebarCollapseContext);
}
