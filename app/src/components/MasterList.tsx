import { useMemo, useState } from "react";
import { NavLink, Outlet } from "react-router-dom";
import { Spinner } from "./primitives";

export type ListItem = { id: number; primary: string; secondary?: string };

/// A master-detail list pane: filterable scrolling list of items on the left (NavLinks to `:id`),
/// with the active detail rendered in the right pane via `<Outlet/>`.
export function MasterList({
  items,
  isLoading,
  error,
  placeholder,
}: {
  items: ListItem[];
  isLoading: boolean;
  error?: unknown;
  placeholder: string;
}) {
  const [filter, setFilter] = useState("");
  const rows = useMemo(() => {
    const q = filter.trim().toLowerCase();
    if (!q) return items;
    return items.filter(
      (it) =>
        it.primary.toLowerCase().includes(q) ||
        (it.secondary ?? "").toLowerCase().includes(q),
    );
  }, [items, filter]);

  return (
    <div className="flex h-full gap-4">
      <aside className="flex w-72 shrink-0 flex-col">
        <input
          className="mb-2 rounded-md border border-slate-700 bg-slate-950 px-2 py-1.5 text-sm text-slate-100 placeholder-slate-600 focus:border-sky-600 focus:outline-none"
          placeholder={placeholder}
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
        />
        {isLoading && <Spinner label="Loading…" />}
        {error ? <p className="text-sm text-rose-400">{String(error)}</p> : null}
        {!isLoading && (
          <p className="px-1 pb-1 text-xs text-slate-500">
            {rows.length}
            {items.length !== rows.length ? ` of ${items.length}` : ""}
          </p>
        )}
        <div className="flex-1 overflow-auto rounded-md border border-slate-800 bg-slate-900/40">
          {rows.slice(0, 2000).map((it) => (
            <NavLink
              key={it.id}
              to={String(it.id)}
              className={({ isActive }) =>
                "flex justify-between border-b border-slate-800/60 px-3 py-1.5 " +
                (isActive
                  ? "bg-sky-900/30 font-semibold text-sky-100"
                  : "text-slate-200 hover:bg-slate-800/40")
              }
            >
              <span className="truncate">{it.primary}</span>
              {it.secondary && (
                <span className="ml-2 shrink-0 text-xs text-slate-400">{it.secondary}</span>
              )}
            </NavLink>
          ))}
          {rows.length > 2000 && (
            <p className="px-3 py-2 text-xs text-slate-500">Showing first 2000 — refine the filter.</p>
          )}
        </div>
      </aside>
      <section className="min-w-0 flex-1 overflow-auto">
        <Outlet />
      </section>
    </div>
  );
}
