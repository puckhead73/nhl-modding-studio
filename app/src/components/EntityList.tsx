import {
  useDeferredValue,
  useEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
  type ReactNode,
} from "react";
import { NavLink, Outlet } from "react-router-dom";
import { useVirtualizer } from "@tanstack/react-virtual";
import { Spinner } from "./primitives";

const ROW_H = 26; // px — compact rows, matching the old studio's dense list.

export type Column<T> = {
  key: string;
  label: string;
  /** CSS grid track width, e.g. `"40px"` | `"1fr"` | `"56px"`. */
  width: string;
  /** Provide to make the column sortable. */
  sort?: (row: T) => string | number;
  render: (row: T) => ReactNode;
  /** Extra classes for the cell wrapper (color/weight), possibly row-dependent. */
  cellClass?: (row: T) => string;
};

/// A reusable master-detail list: a virtualized, sortable, filterable columnar list on the left
/// (NavLinks to each row's `:id`) with the active detail rendered on the right via `<Outlet/>`.
/// Shared by Players / Teams / Arenas so the dense columnar aesthetic and windowing live in one place.
export function EntityList<T>({
  title,
  rows,
  getId,
  columns,
  searchOf,
  searchPlaceholder,
  isLoading,
  error,
  toolbar,
  initialSort,
  total,
  panelWidth,
  onSearchChange,
  getKey,
  pinLast,
}: {
  title: string;
  rows: T[];
  getId: (row: T) => number;
  columns: Column<T>[];
  searchOf: (row: T) => string;
  searchPlaceholder: string;
  isLoading: boolean;
  error?: unknown;
  /** Optional control rendered left of the search box (e.g. a position filter). */
  toolbar?: ReactNode;
  /** Column key to sort by initially (defaults to the first column). */
  initialSort?: string;
  /** Pre-filter total for the `visible / total` count; defaults to `rows.length`. */
  total?: number;
  /** Tailwind width class for the list panel (defaults to `w-80`); widen for long-name entities. */
  panelWidth?: string;
  /** When set, the list is server-filtered: the search box is debounced into this callback and the
   *  built-in client-side text filter is skipped (rows are assumed already filtered). */
  onSearchChange?: (q: string) => void;
  /** Optional stable route/link segment for a row (defaults to `String(getId(row))`). Use when a row
   *  needs a composite key (e.g. `season-team`). */
  getKey?: (row: T) => string;
  /** When set, rows for which this returns `true` are always sorted to the bottom, regardless of the
   *  active sort column/direction (e.g. players whose name failed to decode). Pinned rows keep their
   *  normal relative order among themselves. */
  pinLast?: (row: T) => boolean;
}) {
  const [q, setQ] = useState("");
  const dq = useDeferredValue(q);
  const [sortKey, setSortKey] = useState(initialSort ?? columns[0]?.key);
  const [sortDir, setSortDir] = useState<"asc" | "desc">("asc");
  const controlled = !!onSearchChange;

  // Debounce the search box into the server query (via useDeferredValue) when controlled.
  useEffect(() => {
    if (onSearchChange) onSearchChange(dq);
  }, [dq, onSearchChange]);

  const gridStyle: CSSProperties = {
    display: "grid",
    gridTemplateColumns: columns.map((c) => c.width).join(" "),
    columnGap: "0.5rem",
    alignItems: "center",
  };

  const view = useMemo(() => {
    const needle = dq.trim().toLowerCase();
    // Server-filtered lists (controlled) skip the client-side text filter — the rows are already
    // narrowed by the query — but still sort locally.
    let out = controlled || !needle ? rows : rows.filter((r) => searchOf(r).toLowerCase().includes(needle));
    const col = columns.find((c) => c.key === sortKey);
    if (col?.sort || pinLast) {
      const dir = sortDir === "asc" ? 1 : -1;
      const s = col?.sort;
      out = [...out].sort((a, b) => {
        // Pinned rows sink to the bottom regardless of the active sort column/direction.
        if (pinLast) {
          const pa = pinLast(a) ? 1 : 0;
          const pb = pinLast(b) ? 1 : 0;
          if (pa !== pb) return pa - pb;
        }
        if (!s) return 0;
        const av = s(a);
        const bv = s(b);
        if (typeof av === "number" && typeof bv === "number") return (av - bv) * dir;
        return String(av).localeCompare(String(bv)) * dir;
      });
    }
    return out;
  }, [rows, dq, sortKey, sortDir, columns, searchOf, pinLast]);

  const scrollRef = useRef<HTMLDivElement>(null);
  const virt = useVirtualizer({
    count: view.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => ROW_H,
    overscan: 16,
  });

  const toggleSort = (k: string) => {
    if (k === sortKey) setSortDir((d) => (d === "asc" ? "desc" : "asc"));
    else {
      setSortKey(k);
      setSortDir("asc");
    }
  };
  const arrow = (k: string) => (sortKey === k ? (sortDir === "asc" ? " ↑" : " ↓") : "");

  return (
    <div className="flex h-full gap-4">
      <aside className={"flex shrink-0 flex-col " + (panelWidth ?? "w-80")}>
        <div className="mb-2 flex items-baseline justify-between">
          <h2 className="text-sm font-semibold text-slate-100">{title}</h2>
          <span className="text-xs text-slate-500">
            {view.length.toLocaleString()} / {(total ?? rows.length).toLocaleString()}
          </span>
        </div>

        <div className="mb-2 flex gap-2">
          {toolbar}
          <input
            value={q}
            onChange={(e) => setQ(e.target.value)}
            placeholder={searchPlaceholder}
            className="min-w-0 flex-1 rounded-md border border-slate-700 bg-slate-950 px-2 py-1.5 text-xs text-slate-100 placeholder-slate-600 focus:border-sky-600 focus:outline-none"
          />
        </div>

        {/* Sortable column header (aligned to the rows via the shared grid). */}
        <div
          style={gridStyle}
          className="rounded-t-md border border-slate-700 bg-slate-800/40 px-2 py-1 text-[10px] font-semibold uppercase tracking-wide text-slate-400"
        >
          {columns.map((c) =>
            c.sort ? (
              <button
                key={c.key}
                type="button"
                onClick={() => toggleSort(c.key)}
                className="text-left hover:text-slate-200"
              >
                {c.label}
                {arrow(c.key)}
              </button>
            ) : (
              <span key={c.key}>{c.label}</span>
            ),
          )}
        </div>

        {isLoading && <Spinner label="Loading…" />}
        {error ? <p className="text-sm text-rose-400">{String(error)}</p> : null}

        <div
          ref={scrollRef}
          className="flex-1 overflow-auto rounded-b-md border border-t-0 border-slate-800 bg-slate-900/40"
        >
          <div style={{ height: virt.getTotalSize(), position: "relative" }}>
            {virt.getVirtualItems().map((v) => {
              const row = view[v.index];
              const seg = getKey ? getKey(row) : String(getId(row));
              return (
                <NavLink
                  key={seg}
                  to={seg}
                  style={{
                    ...gridStyle,
                    position: "absolute",
                    top: 0,
                    left: 0,
                    width: "100%",
                    height: v.size,
                    transform: `translateY(${v.start}px)`,
                  }}
                  className={({ isActive }) =>
                    "border-b border-slate-800/50 px-2 text-xs " +
                    (isActive ? "bg-sky-900/30 text-sky-100" : "text-slate-200 hover:bg-slate-800/40")
                  }
                >
                  {columns.map((c) => (
                    <span key={c.key} className={"min-w-0 truncate " + (c.cellClass?.(row) ?? "")}>
                      {c.render(row)}
                    </span>
                  ))}
                </NavLink>
              );
            })}
          </div>
        </div>
      </aside>
      <section className="min-w-0 flex-1 overflow-auto">
        <Outlet />
      </section>
    </div>
  );
}
