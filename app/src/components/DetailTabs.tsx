import { useState, type ReactNode } from "react";

export type DetailTab = { key: string; label: string; badge?: ReactNode; content: ReactNode };

/// A local subtab strip for a detail pane (Player / Team / Arena). State is component-local — detail
/// subtabs are a within-record view switch, not app navigation, so they stay out of the URL.
export function DetailTabs({ tabs, initial }: { tabs: DetailTab[]; initial?: string }) {
  const [active, setActive] = useState(initial ?? tabs[0]?.key);
  const current = tabs.find((t) => t.key === active) ?? tabs[0];

  return (
    <div>
      <div role="tablist" className="flex flex-wrap items-center gap-1 border-b border-slate-800">
        {tabs.map((t) => {
          const on = t.key === current?.key;
          return (
            <button
              key={t.key}
              type="button"
              role="tab"
              aria-selected={on}
              onClick={() => setActive(t.key)}
              className={
                "flex items-center gap-1.5 rounded-t-md border-b-2 px-3 py-1.5 text-sm font-medium transition-colors " +
                (on
                  ? "border-sky-500 text-sky-100"
                  : "border-transparent text-slate-400 hover:text-slate-100")
              }
            >
              {t.label}
              {t.badge != null && (
                <span className="rounded-full bg-slate-800 px-1.5 text-[10px] text-slate-400">
                  {t.badge}
                </span>
              )}
            </button>
          );
        })}
      </div>
      <div className="pt-3">{current?.content}</div>
    </div>
  );
}
