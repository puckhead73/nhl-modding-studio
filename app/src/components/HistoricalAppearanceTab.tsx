import { useMemo, useState } from "react";
import { keepPreviousData, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { openUrl } from "@tauri-apps/plugin-opener";
import { call } from "../lib/client";
import { Spinner } from "./primitives";
import type { HistoricalPersonDto } from "../bindings/HistoricalPersonDto";
import type { PortraitCandidateDto } from "../bindings/PortraitCandidateDto";

/// The Historical player's Appearance tab — the portrait selector over the supplementary portrait
/// library (`graphical database/portraits`). Candidates come from one of four identity tiers, exact
/// by default, loosening as the operator chooses; a free-text search covers everything else. The
/// choice is recorded per corpus person (beside the corpus) and is what a later Roster Builder import
/// writes into the target title.
export function HistoricalAppearanceTab({ p }: { p: HistoricalPersonDto }) {
  const status = useQuery({
    queryKey: ["portrait_library_status"],
    queryFn: () => call("portrait_library_status", {}),
    staleTime: 60_000,
  });

  if (status.isLoading) return <Spinner label="Indexing the portrait library…" />;
  if (status.isError) return <p className="text-sm text-rose-400">{String(status.error)}</p>;
  const st = status.data!;

  return (
    <div className="space-y-6">
      <LibraryStatusBar st={st} />
      {st.available ? (
        <>
          <SelectedPortrait personId={p.person_id} />
          <CandidatePicker p={p} />
          <SearchPicker personId={p.person_id} dob={p.dob} />
        </>
      ) : (
        <p className="text-sm text-slate-400">
          No portrait library was found. Place the compiled tree at the folder above (or point
          Studio at it under “Library location”), then rescan.
        </p>
      )}
    </div>
  );
}

// ------------------------------------------------------------------ status + location

function LibraryStatusBar({
  st,
}: {
  st: Awaited<ReturnType<typeof call<"portrait_library_status">>>;
}) {
  const qc = useQueryClient();
  const [root, setRoot] = useState("");
  const invalidate = () => {
    void qc.invalidateQueries({ queryKey: ["portrait_library_status"] });
    void qc.invalidateQueries({ queryKey: ["hist_portrait_candidates"] });
    void qc.invalidateQueries({ queryKey: ["hist_portrait_selected"] });
    void qc.invalidateQueries({ queryKey: ["portrait_library_search"] });
  };
  const rescan = useMutation({
    mutationFn: () => call("portrait_library_rescan", {}),
    onSuccess: invalidate,
  });
  const setRootM = useMutation({
    mutationFn: (r: string) => call("portrait_library_set_root", { root: r }),
    onSuccess: invalidate,
  });
  const busy = rescan.isPending || setRootM.isPending;

  return (
    <div className="rounded-md border border-slate-800 bg-slate-900/40 px-3 py-2 text-xs text-slate-400">
      <div className="flex flex-wrap items-center gap-x-4 gap-y-1">
        <span>
          <span className="font-semibold text-slate-200">{st.portrait_count.toLocaleString()}</span>{" "}
          portraits · <span className="text-slate-200">{st.person_count.toLocaleString()}</span> people
          {st.selection_count > 0 && (
            <>
              {" · "}
              <span className="text-slate-200">{st.selection_count.toLocaleString()}</span> selected
            </>
          )}
        </span>
        <span>
          {st.backend === "sqlite_index" ? (
            <>
              SQLite index{st.library_version ? ` ${st.library_version}` : ""}
            </>
          ) : st.csv_path ? (
            <>Filename scan + provenance CSV ({st.csv_rows.toLocaleString()} rows)</>
          ) : (
            <span className="text-amber-400/90">
              No index or provenance CSV found — names come from filenames
            </span>
          )}
        </span>
        <button
          type="button"
          disabled={busy}
          onClick={() => rescan.mutate()}
          className="ml-auto rounded border border-slate-700 px-2 py-0.5 text-[11px] text-slate-300 hover:border-sky-600 hover:text-sky-300 disabled:opacity-50"
        >
          {busy ? "Scanning…" : "Rescan"}
        </button>
      </div>
      <details className="mt-1">
        <summary className="cursor-pointer text-[11px] uppercase tracking-wide text-slate-500 hover:text-slate-300">
          Library location{st.warnings.length > 0 ? ` · ${st.warnings.length} warning(s)` : ""}
        </summary>
        <div className="mt-2 space-y-2">
          <p className="break-all font-mono text-[11px] text-slate-500">{st.root}</p>
          <div className="flex gap-2">
            <input
              value={root}
              onChange={(e) => setRoot(e.target.value)}
              placeholder="Override folder (leave empty to use the default beside the corpus)"
              className="flex-1 rounded-md border border-slate-700 bg-slate-950 px-2 py-1 text-xs text-slate-100 focus:border-sky-600 focus:outline-none"
            />
            <button
              type="button"
              disabled={busy}
              onClick={() => setRootM.mutate(root)}
              className="rounded border border-slate-700 px-2 py-0.5 text-[11px] text-slate-300 hover:border-sky-600 hover:text-sky-300 disabled:opacity-50"
            >
              Use folder
            </button>
          </div>
          {setRootM.isError && <p className="text-rose-400">{String(setRootM.error)}</p>}
          {st.warnings.length > 0 && (
            <ul className="max-h-32 space-y-0.5 overflow-auto text-[11px] text-amber-400/80">
              {st.warnings.map((w, i) => (
                <li key={i}>{w}</li>
              ))}
            </ul>
          )}
        </div>
      </details>
    </div>
  );
}

// ------------------------------------------------------------------ current selection

function useSelectPortrait(personId: number) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (relPath: string | null) => call("set_historical_portrait", { personId, relPath }),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ["hist_portrait_selected", personId] });
      void qc.invalidateQueries({ queryKey: ["hist_portrait_candidates", personId] });
      void qc.invalidateQueries({ queryKey: ["portrait_library_status"] });
    },
  });
}

export function useSelectedPortrait(personId: number) {
  return useQuery({
    queryKey: ["hist_portrait_selected", personId],
    queryFn: () => call("historical_portrait_selected", { personId }),
    enabled: Number.isFinite(personId),
  });
}

function SelectedPortrait({ personId }: { personId: number }) {
  const sel = useSelectedPortrait(personId);
  const select = useSelectPortrait(personId);
  const c = sel.data ?? null;
  return (
    <section>
      <SectionTitle>Selected Portrait</SectionTitle>
      {c ? (
        <div className="flex items-start gap-4">
          <PortraitImage relPath={c.rel_path} size={256} className="h-40 w-40" alt={c.display_name} />
          <div className="min-w-0 flex-1 space-y-1 text-sm">
            <p className="font-semibold text-slate-100">{c.display_name}</p>
            <CandidateMeta c={c} />
            <button
              type="button"
              disabled={select.isPending}
              onClick={() => select.mutate(null)}
              className="mt-2 rounded border border-slate-700 px-2 py-0.5 text-[11px] text-slate-300 hover:border-rose-600 hover:text-rose-300 disabled:opacity-50"
            >
              Clear selection
            </button>
            {select.isError && <p className="text-xs text-rose-400">{String(select.error)}</p>}
          </div>
        </div>
      ) : (
        <p className="text-sm text-slate-500">
          No portrait selected yet. Pick one from the matches below, or search the library.
        </p>
      )}
    </section>
  );
}

// ------------------------------------------------------------------ identity-tier matches

const MODES: { key: string; label: string; hint: string }[] = [
  { key: "exact", label: "Exact match", hint: "Last name + first name + date of birth (default)." },
  { key: "last_dob", label: "Last name + DOB", hint: "Ignores first-name variations." },
  { key: "last_only", label: "Last name only", hint: "Ignores first name and date of birth." },
  { key: "dob_only", label: "DOB only", hint: "Ignores both names — covers surname spelling variations." },
];

function CandidatePicker({ p }: { p: HistoricalPersonDto }) {
  const [mode, setMode] = useState("exact");
  const q = useQuery({
    queryKey: ["hist_portrait_candidates", p.person_id, mode],
    queryFn: () => call("historical_portrait_candidates", { personId: p.person_id, mode, limit: 300 }),
    placeholderData: keepPreviousData,
  });
  const select = useSelectPortrait(p.person_id);
  const selectedPath = q.data?.selected?.rel_path ?? null;
  const hint = MODES.find((m) => m.key === mode)?.hint;
  const noDob = !p.dob || p.dob === "?";

  return (
    <section>
      <SectionTitle>
        Matches{q.data ? ` (${q.data.total.toLocaleString()})` : ""}
      </SectionTitle>
      <div className="mb-3 flex flex-wrap items-center gap-2">
        <div role="radiogroup" className="flex flex-wrap gap-1">
          {MODES.map((m) => {
            const on = m.key === mode;
            const disabled = noDob && m.key !== "last_only";
            return (
              <button
                key={m.key}
                type="button"
                role="radio"
                aria-checked={on}
                disabled={disabled}
                title={disabled ? "This person has no date of birth on record" : m.hint}
                onClick={() => setMode(m.key)}
                className={
                  "rounded-md border px-2.5 py-1 text-xs font-medium transition-colors disabled:cursor-default disabled:opacity-40 " +
                  (on
                    ? "border-sky-600 bg-sky-900/40 text-sky-100"
                    : "border-slate-700 text-slate-300 hover:border-slate-500 hover:text-slate-100")
                }
              >
                {m.label}
              </button>
            );
          })}
        </div>
        {hint && <span className="text-[11px] text-slate-500">{hint}</span>}
      </div>
      <p className="mb-2 text-[11px] text-slate-500">
        Matching against <span className="text-slate-300">{p.last_name}, {p.first_name}</span>
        {noDob ? " (no DOB on record)" : <> · <span className="text-slate-300">{p.dob}</span></>}
      </p>
      {q.isLoading ? (
        <Spinner label="Matching…" />
      ) : q.isError ? (
        <p className="text-sm text-rose-400">{String(q.error)}</p>
      ) : q.data && q.data.candidates.length > 0 ? (
        <>
          <CandidateGrid
            candidates={q.data.candidates}
            selectedPath={selectedPath}
            onSelect={(rel) => select.mutate(rel)}
            busy={select.isPending}
          />
          {q.data.total > q.data.candidates.length && (
            <p className="mt-2 text-[11px] text-slate-500">
              Showing the first {q.data.candidates.length} of {q.data.total.toLocaleString()} — use Search to narrow.
            </p>
          )}
        </>
      ) : (
        <p className="text-sm text-slate-500">
          No portraits match at this tier. Try a looser tier, or search below.
        </p>
      )}
    </section>
  );
}

// ------------------------------------------------------------------ search

function SearchPicker({ personId, dob }: { personId: number; dob: string }) {
  const [query, setQuery] = useState("");
  const [dobQ, setDobQ] = useState("");
  const [submitted, setSubmitted] = useState<{ query: string; dob: string } | null>(null);
  const q = useQuery({
    queryKey: ["portrait_library_search", submitted?.query ?? "", submitted?.dob ?? ""],
    queryFn: () =>
      call("portrait_library_search", {
        query: submitted!.query,
        dob: submitted!.dob.trim() ? submitted!.dob.trim() : null,
        limit: 200,
      }),
    enabled: submitted != null && (submitted.query.trim() !== "" || submitted.dob.trim() !== ""),
    placeholderData: keepPreviousData,
  });
  const sel = useSelectedPortrait(personId);
  const select = useSelectPortrait(personId);
  const inputCls =
    "rounded-md border border-slate-700 bg-slate-950 px-2 py-1 text-xs text-slate-100 focus:border-sky-600 focus:outline-none";

  return (
    <section>
      <SectionTitle>Search the Library</SectionTitle>
      <form
        className="mb-3 flex flex-wrap items-center gap-2"
        onSubmit={(e) => {
          e.preventDefault();
          setSubmitted({ query, dob: dobQ });
        }}
      >
        <input
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Name, alias, slug or source…"
          className={inputCls + " w-64"}
        />
        <input
          value={dobQ}
          onChange={(e) => setDobQ(e.target.value)}
          placeholder="DOB (optional: 1979-11-14, 14_11_1979 or 1979)"
          className={inputCls + " w-72"}
        />
        {dob && dob !== "?" && (
          <button
            type="button"
            onClick={() => setDobQ(dob)}
            className="rounded border border-slate-700 px-2 py-0.5 text-[11px] text-slate-300 hover:border-sky-600 hover:text-sky-300"
            title="Fill in this person's date of birth"
          >
            Use {dob}
          </button>
        )}
        <button
          type="submit"
          className="rounded-md border border-sky-700 bg-sky-900/40 px-3 py-1 text-xs font-medium text-sky-100 hover:bg-sky-900/70"
        >
          Search
        </button>
      </form>
      {submitted == null ? (
        <p className="text-[11px] text-slate-500">
          Every word must appear in the portrait's names, aliases, file stem or source; the DOB filter
          accepts a full date or a bare year.
        </p>
      ) : q.isLoading ? (
        <Spinner label="Searching…" />
      ) : q.isError ? (
        <p className="text-sm text-rose-400">{String(q.error)}</p>
      ) : q.data && q.data.length > 0 ? (
        <>
          <CandidateGrid
            candidates={q.data}
            selectedPath={sel.data?.rel_path ?? null}
            onSelect={(rel) => select.mutate(rel)}
            busy={select.isPending}
          />
          {q.data.length >= 200 && (
            <p className="mt-2 text-[11px] text-slate-500">Showing the first 200 hits — add a word or a DOB.</p>
          )}
        </>
      ) : (
        <p className="text-sm text-slate-500">No portraits found.</p>
      )}
    </section>
  );
}

// ------------------------------------------------------------------ shared pieces

/** Candidates grouped by person stem so one person's variants sit together. */
function CandidateGrid({
  candidates,
  selectedPath,
  onSelect,
  busy,
}: {
  candidates: PortraitCandidateDto[];
  selectedPath: string | null;
  onSelect: (relPath: string) => void;
  busy: boolean;
}) {
  const groups = useMemo(() => {
    const m = new Map<string, PortraitCandidateDto[]>();
    for (const c of candidates) {
      const g = m.get(c.stem);
      if (g) g.push(c);
      else m.set(c.stem, [c]);
    }
    return [...m.values()];
  }, [candidates]);

  return (
    <div className="space-y-4">
      {groups.map((g) => {
        const head = g[0];
        return (
          <div key={head.stem}>
            {groups.length > 1 && (
              <div className="mb-1 flex flex-wrap items-baseline gap-x-2 text-xs">
                <span className="font-semibold text-slate-200">{head.display_name}</span>
                {head.dob && <span className="tabular-nums text-slate-500">{head.dob}</span>}
                <span className="text-slate-600">
                  {g.length} {g.length === 1 ? "portrait" : "portraits"}
                </span>
              </div>
            )}
            <div className="flex flex-wrap gap-2">
              {g.map((c) => (
                <CandidateTile
                  key={c.rel_path}
                  c={c}
                  selected={c.rel_path === selectedPath}
                  onSelect={() => onSelect(c.rel_path)}
                  busy={busy}
                />
              ))}
            </div>
          </div>
        );
      })}
    </div>
  );
}

function CandidateTile({
  c,
  selected,
  onSelect,
  busy,
}: {
  c: PortraitCandidateDto;
  selected: boolean;
  onSelect: () => void;
  busy: boolean;
}) {
  return (
    <div
      className={
        "w-36 rounded-md border p-1.5 transition-colors " +
        (selected ? "border-emerald-500 bg-emerald-900/20" : "border-slate-800 bg-slate-900/40")
      }
    >
      <button
        type="button"
        disabled={busy || selected}
        onClick={onSelect}
        title={selected ? "Selected" : `Select ${c.display_name}`}
        className="block w-full disabled:cursor-default"
      >
        <PortraitImage relPath={c.rel_path} size={160} className="h-32 w-full" alt={c.display_name} />
      </button>
      <div className="mt-1 space-y-0.5 text-[11px] leading-tight">
        <p className="truncate font-medium text-slate-200" title={c.display_name}>
          {c.display_name}
        </p>
        <CandidateMeta c={c} compact />
      </div>
    </div>
  );
}

/** DOB · variant · tier · source link — the per-candidate detail line. */
function CandidateMeta({ c, compact }: { c: PortraitCandidateDto; compact?: boolean }) {
  const source = c.source ?? null;
  const isUrl = source != null && /^https?:\/\//i.test(source);
  const sourceLabel = isUrl ? hostOf(source) : source;
  return (
    <div className={compact ? "space-y-0.5 text-slate-500" : "space-y-0.5 text-xs text-slate-400"}>
      <p className="flex flex-wrap gap-x-1.5">
        {c.dob && <span className="tabular-nums">{c.dob}</span>}
        {c.variant > 0 && <span>#{c.variant}</span>}
        {c.source_type && <span className="text-slate-500">{c.source_type}</span>}
        {c.tier && <span className="text-slate-600">{tierLabel(c.tier)}</span>}
      </p>
      {c.identity_status === "ambiguous_name_boundary" && (
        <p className="text-amber-400/80" title="The index could not split this compound name on its own; Studio resolved the split against this player's name">
          Compound name resolved by Studio
        </p>
      )}
      {c.identity_status === "malformed_identity" && (
        <p className="text-rose-400/80" title="The index flagged this filename's identity as malformed">
          Malformed identity
        </p>
      )}
      {!compact && c.games && <p className="truncate" title={c.games}>Titles: {c.games.split(";").join(", ")}</p>}
      {sourceLabel ? (
        isUrl ? (
          <button
            type="button"
            onClick={() => void openUrl(source!)}
            className="truncate text-left text-sky-400/90 hover:text-sky-300"
            title={source!}
          >
            {sourceLabel}
          </button>
        ) : (
          <p className="truncate" title={source!}>
            {sourceLabel}
          </p>
        )
      ) : (
        !compact && <p className="text-slate-600">No source recorded</p>
      )}
      {!compact && <p className="break-all font-mono text-[10px] text-slate-600">{c.rel_path}</p>}
    </div>
  );
}

/** One library image, fetched as a data URL and cached for the session. */
export function PortraitImage({
  relPath,
  size,
  className,
  alt,
}: {
  relPath: string;
  size?: number;
  className?: string;
  alt: string;
}) {
  const img = useQuery({
    queryKey: ["portrait_library_image", relPath, size ?? 0],
    queryFn: () => call("portrait_library_image", { relPath, maxSize: size }),
    staleTime: Infinity,
  });
  return (
    <div
      className={
        "grid place-items-center overflow-hidden rounded border border-slate-800 bg-slate-950/60 " +
        (className ?? "")
      }
    >
      {img.data ? (
        <img src={img.data} alt={alt} className="h-full w-full object-contain" loading="lazy" />
      ) : img.isError ? (
        <span className="px-1 text-center text-[10px] text-rose-400">unreadable</span>
      ) : (
        <span className="text-[10px] text-slate-600">…</span>
      )}
    </div>
  );
}

function SectionTitle({ children }: { children: React.ReactNode }) {
  return (
    <h3 className="mb-2 border-b border-slate-800 pb-1 text-[11px] font-bold uppercase tracking-wider text-slate-400">
      {children}
    </h3>
  );
}

function tierLabel(t: string): string {
  return MODES.find((m) => m.key === t)?.label ?? t;
}

function hostOf(url: string): string {
  try {
    return new URL(url).hostname.replace(/^www\./, "");
  } catch {
    return url;
  }
}
