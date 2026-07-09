// Shared UI primitives — the slate dark-theme vocabulary used across the app.
// Adapted to our stack; styling mirrors the desktop chrome (slate base, sky
// primary, emerald/amber/rose semantic accents).

import type { ButtonHTMLAttributes, ReactNode } from "react";

/** A panel/card surface. */
export function Card({
  title,
  subtitle,
  actions,
  children,
  className,
}: {
  title?: ReactNode;
  subtitle?: ReactNode;
  actions?: ReactNode;
  children?: ReactNode;
  className?: string;
}) {
  return (
    <div
      className={
        "rounded-lg border border-slate-800 bg-slate-900/60 shadow-sm " +
        (className ?? "")
      }
    >
      {(title || actions) && (
        <header className="flex items-start justify-between gap-4 border-b border-slate-800 px-4 py-3">
          <div>
            {title && <h2 className="text-sm font-semibold text-slate-100">{title}</h2>}
            {subtitle && <p className="mt-0.5 text-xs text-slate-400">{subtitle}</p>}
          </div>
          {actions && <div className="flex items-center gap-2">{actions}</div>}
        </header>
      )}
      <div className="px-4 py-3">{children}</div>
    </div>
  );
}

/** A small labelled stat tile. */
export function Stat({ label, value, hint }: { label: string; value: ReactNode; hint?: string }) {
  return (
    <div className="rounded-md border border-slate-800 bg-slate-900/40 px-3 py-2">
      <div className="text-[11px] uppercase tracking-wide text-slate-500">{label}</div>
      <div className="mt-1 font-mono text-base text-slate-100">{value}</div>
      {hint && <div className="mt-0.5 text-xs text-slate-400">{hint}</div>}
    </div>
  );
}

export type Tone = "neutral" | "ok" | "warn" | "error" | "info";

const TONE_CLASS: Record<Tone, string> = {
  ok: "bg-emerald-900/40 text-emerald-300 border-emerald-800",
  warn: "bg-amber-900/40 text-amber-300 border-amber-800",
  error: "bg-rose-900/40 text-rose-300 border-rose-800",
  info: "bg-sky-900/40 text-sky-300 border-sky-800",
  neutral: "bg-slate-800/60 text-slate-300 border-slate-700",
};

/** A status/count pill. */
export function Pill({ children, tone = "neutral" }: { children: ReactNode; tone?: Tone }) {
  return (
    <span
      className={
        "inline-flex items-center rounded-full border px-2 py-0.5 text-[11px] font-medium " +
        TONE_CLASS[tone]
      }
    >
      {children}
    </span>
  );
}

type ButtonProps = ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: "primary" | "secondary" | "ghost";
};

const BUTTON_VARIANT = {
  primary: "bg-sky-600 hover:bg-sky-500 text-white border-sky-500/40",
  secondary: "bg-slate-800 hover:bg-slate-700 text-slate-100 border-slate-700",
  ghost: "bg-transparent hover:bg-slate-800 text-slate-300 border-transparent",
} as const;

export function Button({ variant = "primary", className, children, ...rest }: ButtonProps) {
  return (
    <button
      className={
        "inline-flex items-center justify-center rounded-md border px-3 py-1.5 text-sm font-medium transition-colors disabled:cursor-not-allowed disabled:opacity-50 " +
        BUTTON_VARIANT[variant] +
        " " +
        (className ?? "")
      }
      {...rest}
    >
      {children}
    </button>
  );
}

/** A red error banner. */
export function ErrorBanner({ title, message }: { title: string; message?: ReactNode }) {
  return (
    <div className="rounded-md border border-rose-800 bg-rose-950/60 p-3 text-rose-100">
      <div className="text-sm font-semibold">{title}</div>
      {message && <div className="mt-1 text-xs text-rose-200/90">{message}</div>}
    </div>
  );
}

/** A dashed empty-state box. */
export function EmptyState({ title, hint }: { title: string; hint?: ReactNode }) {
  return (
    <div className="rounded-md border border-dashed border-slate-800 bg-slate-900/30 px-4 py-8 text-center">
      <div className="text-sm font-medium text-slate-200">{title}</div>
      {hint && <div className="mt-1 text-xs text-slate-400">{hint}</div>}
    </div>
  );
}

/** A small colored status dot (green/amber/neutral). */
export function StatusDot({ ok, warn }: { ok?: boolean; warn?: boolean }) {
  const cls = ok ? "bg-emerald-400" : warn ? "bg-amber-400" : "bg-slate-600";
  return <span className={`mr-1 inline-block h-1.5 w-1.5 rounded-full ${cls}`} />;
}

/** Inline spinner with an optional label. */
export function Spinner({ label }: { label?: string }) {
  return (
    <div className="flex items-center gap-2 text-xs text-slate-400">
      <span className="h-3 w-3 animate-spin rounded-full border-2 border-slate-600 border-t-slate-200" />
      {label}
    </div>
  );
}

/** A scrolling page body region. */
export function PageScroll({ children }: { children: ReactNode }) {
  return <div className="min-h-0 flex-1 overflow-y-auto px-6 py-6">{children}</div>;
}

/** Centered "nothing selected yet" prompt for an empty master-detail right pane. */
export function SelectPrompt({ what }: { what: string }) {
  return (
    <div className="grid h-full place-items-center text-center">
      <div>
        <p className="text-sm font-semibold text-slate-200">Select a {what}</p>
        <p className="mt-1 text-xs text-slate-500">
          Pick a row from the list on the left to view the full detail.
        </p>
      </div>
    </div>
  );
}
