// A navigation *history trail* ("breadcrumbs") — the ordered set of distinct views the user has
// clicked through, rendered under the primary tabs so they can jump back to any of them.
//
// Semantics are a stack, not a URL hierarchy: visiting a new view pushes it; revisiting one already
// in the trail truncates everything after it (you went "back"). Each crumb's label defaults to a
// name derived from the route, but detail pages report their resolved entity name via
// `useCrumbLabel(...)` so the trail reads "Teams › Detroit Red Wings › …" rather than raw ids.

import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { useLocation } from "react-router-dom";
import { SECTIONS } from "../nav";

export type Crumb = { path: string; label: string };

type Ctx = {
  trail: Crumb[];
  setLabel: (path: string, label: string) => void;
};

const BreadcrumbContext = createContext<Ctx | null>(null);

/** Cap the trail so a long session doesn't grow an endless bar; oldest crumbs fall off. */
const MAX = 8;

/** Redirect-only paths that immediately bounce elsewhere — never worth a crumb of their own. */
const IGNORED = new Set(["/app", "/app/", "/app/database", "/app/historical"]);

export function BreadcrumbProvider({ children }: { children: ReactNode }) {
  const { pathname } = useLocation();
  const [paths, setPaths] = useState<string[]>([]);
  const [labels, setLabels] = useState<Record<string, string>>({});

  useEffect(() => {
    if (IGNORED.has(pathname)) return;
    setPaths((prev) => {
      if (prev[prev.length - 1] === pathname) return prev; // already the current view
      const at = prev.indexOf(pathname);
      if (at !== -1) return prev.slice(0, at + 1); // revisited an earlier view → drop forward history
      const next = [...prev, pathname];
      return next.length > MAX ? next.slice(next.length - MAX) : next;
    });
  }, [pathname]);

  // Pages report their resolved title here; guarded so identical writes don't re-render.
  const setLabel = useCallback((path: string, label: string) => {
    setLabels((prev) => (prev[path] === label ? prev : { ...prev, [path]: label }));
  }, []);

  const trail = useMemo(
    () => paths.map((p) => ({ path: p, label: labels[p] ?? deriveLabel(p) })),
    [paths, labels],
  );

  const value = useMemo<Ctx>(() => ({ trail, setLabel }), [trail, setLabel]);
  return <BreadcrumbContext.Provider value={value}>{children}</BreadcrumbContext.Provider>;
}

/** The current history trail (oldest → current), for the breadcrumb bar. */
export function useBreadcrumbs(): Crumb[] {
  return useCtx().trail;
}

/** Report a human-readable title for the *current* view (e.g. a resolved player/team name). No-ops
 *  while `label` is undefined (data still loading), so it's safe to call before the early returns. */
export function useCrumbLabel(label: string | null | undefined) {
  const { setLabel } = useCtx();
  const { pathname } = useLocation();
  useEffect(() => {
    const trimmed = label?.trim();
    if (trimmed) setLabel(pathname, trimmed);
  }, [label, pathname, setLabel]);
}

function useCtx(): Ctx {
  const ctx = useContext(BreadcrumbContext);
  if (!ctx) throw new Error("breadcrumbs hooks must be used within a BreadcrumbProvider");
  return ctx;
}

/** Friendly names for leaf routes that aren't section subtabs. */
const PRETTY: Record<string, string> = {
  dashboard: "Dashboard",
  settings: "Settings",
  assets: "Assets",
  builds: "Builds",
  pending: "Pending Edits",
  transactions: "Transactions",
};

/** Best-effort single-name label for a view from its `/app/...` path (fallback until a page reports
 *  a resolved title). Section → subtab → detail-id, mirroring the nav model. */
function deriveLabel(pathname: string): string {
  const segs = pathname
    .replace(/^\/app\/?/, "")
    .split("/")
    .filter(Boolean);
  if (segs.length === 0) return "Home";
  const [sectionKey, sub] = segs;
  const section = SECTIONS.find((s) => s.key === sectionKey);
  if (segs.length >= 3) return `#${segs[segs.length - 1]}`; // detail: id under a subtab
  if (segs.length === 2) {
    const match = section?.subtabs.find((st) => st.to === `${sectionKey}/${sub}`);
    return match?.label ?? PRETTY[sub] ?? titleCase(sub);
  }
  return section?.label ?? PRETTY[sectionKey] ?? titleCase(sectionKey);
}

function titleCase(s: string): string {
  return s.charAt(0).toUpperCase() + s.slice(1);
}
