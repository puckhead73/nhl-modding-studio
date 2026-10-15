import { useEffect, useMemo, useRef, useState } from "react";
import { keepPreviousData, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { call } from "../lib/client";
import { Button, Spinner } from "../components/primitives";
import { ComputedRatings } from "../components/ComputedRatings";
import type { FormulaProfileDto } from "../bindings/FormulaProfileDto";
import type { FormulaCatalogDto } from "../bindings/FormulaCatalogDto";

type Pos = "forward" | "defenseman" | "goalie";
type Component = [string, number];

/// Drag payload MIME: the component key (a raw FHM `attrs_json` key or a `prebuilt:<key>` ref).
const INPUT_MIME = "application/x-formula-input";

/// The Formulas page — edit the FHM→EA rating profile (per position, per EA attribute: weighted FHM
/// components + prebuilts), toggle the normalisation mode, save/load named profiles, and preview the
/// result live against a chosen historical player-season. The working profile is the persistent
/// *active* formula: every edit auto-saves next to the corpus so it survives navigation and drives
/// the computed ratings shown elsewhere (player detail).
export function FormulasPage() {
  const qc = useQueryClient();
  const catalog = useQuery({ queryKey: ["formula_catalog"], queryFn: () => call("formula_catalog", {}) });
  const profiles = useQuery({ queryKey: ["formula_profiles"], queryFn: () => call("list_formula_profiles", {}) });
  const active = useQuery({ queryKey: ["active_formula"], queryFn: () => call("get_active_formula", {}) });

  const [profile, setProfile] = useState<FormulaProfileDto | null>(null);
  const [pos, setPos] = useState<Pos>("forward");
  const [saveName, setSaveName] = useState("");
  const [status, setStatus] = useState<string | null>(null);
  // After saving a profile the user is asked whether to run the (expensive) Legends precompute now.
  // Declining parks the debt in `ea_formula_state.json`, where the app-level banner picks it up.
  const [askCalc, setAskCalc] = useState<string | null>(null);

  // Which profile the app is working under, and whether its Legends ratings are current.
  const legends = useQuery({
    queryKey: ["legends_status"],
    queryFn: () => call("legends_status", {}),
    retry: false,
  });
  const activeProfile = legends.data?.active_profile ?? "";

  const runCalc = useMutation({
    mutationFn: () => call("compute_legends_ratings", {}),
    onSuccess: async (r) => {
      setAskCalc(null);
      setStatus(`Legends ratings rebuilt — ${r.rated.toLocaleString()} players in ${(r.elapsed_ms / 1000).toFixed(1)}s.`);
      await qc.invalidateQueries({ queryKey: ["legends_status"] });
      await qc.invalidateQueries({ queryKey: ["legends_teams"] });
    },
    onError: (e) => setStatus(`Legends calculation failed: ${e}`),
  });

  // Seed the editor from the persisted active formula once it arrives. If that read fails (e.g. a
  // corrupt `ea_formulas.json`), fall back to the packaged default so the page still loads.
  useEffect(() => {
    if (profile) return;
    if (active.data) setProfile(active.data);
    else if (active.isError) call("formula_default", {}).then(setProfile).catch(() => {});
  }, [profile, active.data, active.isError]);

  // Persist edits to the active formula (debounced so weight-typing doesn't thrash the disk). The
  // cached query is kept in sync so other pages (player detail) pick the change up immediately.
  const saveTimer = useRef<number | null>(null);
  const commit = (next: FormulaProfileDto) => {
    setProfile(next);
    qc.setQueryData(["active_formula"], next);
    if (saveTimer.current != null) window.clearTimeout(saveTimer.current);
    saveTimer.current = window.setTimeout(() => {
      call("save_active_formula", { profile: next })
        .then(() => setStatus("Auto-saved."))
        .catch((e) => setStatus(`Save failed: ${e}`));
    }, 300);
  };

  const keyLabels = useMemo(() => labelMap(catalog.data), [catalog.data]);

  if (!profile || catalog.isLoading) return <Spinner label="Loading formulas…" />;
  if (catalog.isError) return <p className="text-rose-400">{String(catalog.error)}</p>;
  const cat = catalog.data!;

  const eaAttrs = pos === "goalie" ? cat.goalie_ea_attrs : cat.skater_ea_attrs;

  const setComponents = (ea: string, comps: Component[]) => {
    const bucket = { ...profile[pos], [ea]: comps };
    if (comps.length === 0) delete bucket[ea];
    commit({ ...profile, [pos]: bucket });
  };
  // Wire an input key onto an EA attribute (from the sidebar drop), ignoring duplicates.
  const addInput = (ea: string, key: string) => {
    const current = (profile[pos][ea] ?? []) as Component[];
    if (!key || current.some((c) => c[0] === key)) return;
    setComponents(ea, [...current, [key, 1.0]]);
  };

  // Switching profiles switches what Legends ranks on, so the backend re-checks whether the newly
  // active profile has current ratings and re-raises the pending banner if it doesn't.
  const load = async (name: string) => {
    const next = name ? await call("load_formula_profile", { name }) : await call("formula_default", {});
    commit(next);
    setSaveName(name);
    if (name) await call("set_active_profile", { name });
    await qc.invalidateQueries({ queryKey: ["legends_status"] });
    setStatus(name ? `Loaded "${name}".` : "Reset to built-in default.");
  };
  const save = async () => {
    const name = saveName.trim();
    if (!name) return;
    await call("save_formula_profile", { name, profile });
    await call("set_active_profile", { name });
    await qc.invalidateQueries({ queryKey: ["formula_profiles"] });
    await qc.invalidateQueries({ queryKey: ["legends_status"] });
    setStatus(`Saved "${name}".`);
    // New weightings mean new Legends ratings — offer to rebuild them straight away.
    setAskCalc(name);
  };
  const declineCalc = async () => {
    await call("set_legends_pending", { pending: true, dismissed: false });
    await qc.invalidateQueries({ queryKey: ["legends_status"] });
    setAskCalc(null);
    setStatus("Saved. Legends ratings still need calculating — the banner will keep reminding you.");
  };
  const del = async (name: string) => {
    await call("delete_formula_profile", { name });
    await qc.invalidateQueries({ queryKey: ["formula_profiles"] });
    setStatus(`Deleted "${name}".`);
  };

  return (
    <div className="flex h-full gap-4">
      {/* Input palette */}
      <InputCatalog catalog={cat} />

      {/* Editor */}
      <div className="flex min-w-0 flex-1 flex-col">
        <div className="mb-3 flex flex-wrap items-center gap-2">
          <h2 className="text-sm font-semibold text-slate-100">Formulas</h2>
          <span className="text-xs text-slate-500">FHM → EA rating mapping</span>
          <div className="ml-auto flex items-center gap-2">
            <label className="text-[11px] uppercase tracking-wide text-slate-500">Normalise</label>
            <select
              value={profile.normalisation_mode}
              onChange={(e) => commit({ ...profile, normalisation_mode: e.target.value })}
              className="rounded-md border border-slate-700 bg-slate-950 px-2 py-1 text-xs text-slate-100"
            >
              <option value="nominal">Nominal (÷20)</option>
              <option value="position_max">Position max</option>
            </select>
          </div>
        </div>

        {/* Profile load/save row */}
        <div className="mb-3 flex flex-wrap items-center gap-2 rounded-md border border-slate-800 bg-slate-900/40 px-3 py-2">
          <span className="text-[11px] uppercase tracking-wide text-slate-500">Profile</span>
          <select
            value={activeProfile}
            onChange={(e) => load(e.target.value)}
            className="rounded-md border border-slate-700 bg-slate-950 px-2 py-1 text-xs text-slate-100"
          >
            <option value="">Default (built-in)</option>
            {(profiles.data ?? []).map((n) => (
              <option key={n} value={n}>
                {n}
              </option>
            ))}
          </select>
          <input
            value={saveName}
            onChange={(e) => setSaveName(e.target.value)}
            placeholder="Profile name…"
            className="w-40 rounded-md border border-slate-700 bg-slate-950 px-2 py-1 text-xs text-slate-100 placeholder-slate-600"
          />
          <Button variant="secondary" onClick={save} disabled={!saveName.trim()}>
            Save
          </Button>
          {saveName && (profiles.data ?? []).includes(saveName) && (
            <Button variant="ghost" onClick={() => del(saveName)}>
              Delete
            </Button>
          )}
          {legends.data && (
            <span
              className={
                "text-[11px] " +
                (legends.data.computed ? "text-emerald-400/80" : "text-amber-400/90")
              }
              title={
                legends.data.computed
                  ? `Legends ratings computed ${legends.data.computed_at ?? ""}`
                  : "The Legends builder can't rank players under this profile until you calculate."
              }
            >
              {legends.data.computed
                ? `Legends: ${legends.data.player_count.toLocaleString()} rated`
                : legends.data.stale
                  ? "Legends: stale — recalculate"
                  : "Legends: not calculated"}
            </span>
          )}
          {status && <span className="text-xs text-slate-500">{status}</span>}
        </div>

        {/* Post-save prompt: rebuild the Legends ratings now, or park the debt for the banner. */}
        {askCalc && (
          <div className="mb-3 flex flex-wrap items-center gap-2 rounded-md border border-amber-800/70 bg-amber-950/30 px-3 py-2 text-xs text-amber-100">
            <span>
              Calculate Legends ratings for <strong>{askCalc}</strong> now? It reads every player's
              every season, so it takes a while — but the Legends builder can't rank anyone until
              it's done.
            </span>
            <div className="ml-auto flex items-center gap-2">
              <Button
                onClick={() => runCalc.mutate()}
                disabled={runCalc.isPending}
                className="!bg-amber-600 hover:!bg-amber-500"
              >
                {runCalc.isPending ? "Calculating…" : "Calculate now"}
              </Button>
              <Button variant="ghost" onClick={declineCalc} disabled={runCalc.isPending}>
                Later
              </Button>
            </div>
          </div>
        )}

        {/* Position tabs */}
        <div className="mb-2 flex gap-1 border-b border-slate-800">
          {(["forward", "defenseman", "goalie"] as Pos[]).map((p) => (
            <button
              key={p}
              type="button"
              onClick={() => setPos(p)}
              className={
                "rounded-t-md border-b-2 px-3 py-1.5 text-sm font-medium capitalize transition-colors " +
                (p === pos
                  ? "border-sky-500 text-sky-100"
                  : "border-transparent text-slate-400 hover:text-slate-100")
              }
            >
              {p}
            </button>
          ))}
        </div>

        <div className="min-h-0 flex-1 space-y-1 overflow-y-auto pr-2">
          {eaAttrs.map((ea) => (
            <AttrRow
              key={ea}
              ea={ea}
              components={(profile[pos][ea] ?? []) as Component[]}
              catalog={cat}
              keyLabels={keyLabels}
              onChange={(comps) => setComponents(ea, comps)}
              onDropInput={(key) => addInput(ea, key)}
            />
          ))}
        </div>
      </div>

      {/* Live preview */}
      <aside className="flex w-96 shrink-0 flex-col">
        <PreviewPane profile={profile} />
      </aside>
    </div>
  );
}

/** Left palette: draggable prebuilt formulas + the grouped FHM attribute vocabulary. */
function InputCatalog({ catalog }: { catalog: FormulaCatalogDto }) {
  const dragProps = (key: string) => ({
    draggable: true,
    onDragStart: (e: React.DragEvent) => {
      e.dataTransfer.setData(INPUT_MIME, key);
      e.dataTransfer.setData("text/plain", key);
      e.dataTransfer.effectAllowed = "copy";
    },
  });

  return (
    <aside className="flex w-56 shrink-0 flex-col overflow-y-auto rounded-md border border-slate-800 bg-slate-900/40">
      {catalog.prebuilts.length > 0 && (
        <>
          <div className="border-b border-emerald-900/70 bg-emerald-950/50 px-3 py-2 text-[11px] font-semibold uppercase tracking-wide text-emerald-200">
            Custom formulas
          </div>
          <ul className="border-b border-slate-800 px-1 py-1">
            {catalog.prebuilts.map((p) => (
              <li
                key={p.key}
                {...dragProps(p.key)}
                title={p.key}
                className="cursor-grab rounded px-2 py-0.5 text-xs text-emerald-200 hover:bg-emerald-900/40 hover:text-emerald-100 active:cursor-grabbing"
              >
                {p.label}
              </li>
            ))}
          </ul>
        </>
      )}
      <div className="border-b border-slate-800 bg-slate-900 px-3 py-2 text-[11px] font-semibold uppercase tracking-wide text-slate-300">
        FHM attributes
      </div>
      <ul className="px-1 py-1">
        {catalog.fhm_groups.map((g) => (
          <li key={g.group} className="mb-1">
            <div className="rounded-sm bg-slate-800/60 px-2 py-1 text-[10px] font-semibold uppercase tracking-wide text-slate-200">
              {g.group}
            </div>
            <ul>
              {g.attrs.map((a) => (
                <li
                  key={a.key}
                  {...dragProps(a.key)}
                  title={a.key}
                  className="cursor-grab rounded px-2 py-0.5 text-xs text-slate-300 hover:bg-sky-900/30 hover:text-sky-100 active:cursor-grabbing"
                >
                  {a.label}
                </li>
              ))}
            </ul>
          </li>
        ))}
      </ul>
    </aside>
  );
}

/** One EA attribute's editable component list — also a drop target for palette inputs. */
function AttrRow({
  ea,
  components,
  catalog,
  keyLabels,
  onChange,
  onDropInput,
}: {
  ea: string;
  components: Component[];
  catalog: FormulaCatalogDto;
  keyLabels: Map<string, string>;
  onChange: (comps: Component[]) => void;
  onDropInput: (key: string) => void;
}) {
  const [adding, setAdding] = useState(false);
  const [hover, setHover] = useState(false);
  const set = (i: number, weight: number) =>
    onChange(components.map((c, j) => (j === i ? [c[0], weight] : c)));
  const remove = (i: number) => onChange(components.filter((_, j) => j !== i));
  const add = (key: string) => {
    if (key) onChange([...components, [key, 1.0]]);
    setAdding(false);
  };

  return (
    <div className="grid grid-cols-[150px_1fr] items-start gap-2 border-b border-slate-800/50 py-1">
      <span className="pt-1 text-xs font-medium text-slate-300">{ea}</span>
      <div
        onDragOver={(e) => {
          if (e.dataTransfer.types.includes(INPUT_MIME)) {
            e.preventDefault();
            e.dataTransfer.dropEffect = "copy";
            if (!hover) setHover(true);
          }
        }}
        onDragLeave={() => setHover(false)}
        onDrop={(e) => {
          e.preventDefault();
          const key = e.dataTransfer.getData(INPUT_MIME) || e.dataTransfer.getData("text/plain");
          if (key) onDropInput(key);
          setHover(false);
        }}
        className={
          "flex min-h-[28px] flex-wrap items-center gap-1.5 rounded border border-dashed px-1.5 py-1 transition-colors " +
          (hover ? "border-sky-500 bg-sky-900/20" : "border-transparent")
        }
      >
        {components.length === 0 && !adding && (
          <span className="text-xs text-slate-600">{hover ? "drop to add" : "—"}</span>
        )}
        {components.map((c, i) => (
          <span
            key={`${c[0]}-${i}`}
            className="inline-flex items-center gap-1 rounded-full border border-slate-700 bg-slate-900/60 py-0.5 pl-2 pr-1 text-xs"
          >
            <span className="text-slate-300" title={c[0]}>
              {keyLabels.get(c[0]) ?? c[0]}
            </span>
            <input
              type="number"
              step="0.1"
              value={c[1]}
              onChange={(e) => set(i, Number(e.target.value))}
              className="w-12 rounded border border-slate-700 bg-slate-950 px-1 py-0 text-right font-mono text-[11px] text-sky-200 focus:border-sky-600 focus:outline-none"
            />
            <button
              type="button"
              onClick={() => remove(i)}
              className="rounded-full px-1 text-slate-500 hover:text-rose-400"
            >
              ✕
            </button>
          </span>
        ))}
        {adding ? (
          <select
            autoFocus
            defaultValue=""
            onChange={(e) => add(e.target.value)}
            onBlur={() => setAdding(false)}
            className="w-48 rounded border border-slate-700 bg-slate-950 px-1.5 py-0.5 text-xs text-slate-100"
          >
            <option value="" disabled>
              Add input…
            </option>
            <optgroup label="Custom formulas">
              {catalog.prebuilts.map((p) => (
                <option key={p.key} value={p.key}>
                  {p.label}
                </option>
              ))}
            </optgroup>
            {catalog.fhm_groups.map((g) => (
              <optgroup key={g.group} label={g.group}>
                {g.attrs.map((a) => (
                  <option key={a.key} value={a.key}>
                    {a.label}
                  </option>
                ))}
              </optgroup>
            ))}
          </select>
        ) : (
          <button
            type="button"
            onClick={() => setAdding(true)}
            className="rounded-full border border-slate-700 px-2 py-0.5 text-xs text-slate-400 hover:border-sky-700 hover:text-sky-300"
          >
            +
          </button>
        )}
      </div>
    </div>
  );
}

/// The season-dropdown sentinel for the Legends composite (a real season is a numeric year).
const LEGENDS = "legends";

/// Live preview: one card per player, each computed with the working profile, so a weight edit can
/// be judged against several players at once rather than one at a time. Searching adds to the stack
/// instead of replacing it; each card carries its own season / Legends selection, because the
/// interesting comparison is often one player's peak against another's single season.
function PreviewPane({ profile }: { profile: FormulaProfileDto }) {
  const [search, setSearch] = useState("");
  const [personIds, setPersonIds] = useState<number[]>([]);

  const results = useQuery({
    queryKey: ["formula_preview_search", search],
    queryFn: () => call("list_historical_players", { search, goalieFilter: null, limit: 20 }),
    enabled: search.trim().length > 1,
    placeholderData: keepPreviousData,
  });

  const add = (personId: number) => {
    // Adding the same player twice would only duplicate a card, so it's a no-op.
    setPersonIds((ids) => (ids.includes(personId) ? ids : [...ids, personId]));
    setSearch("");
  };
  const remove = (personId: number) => setPersonIds((ids) => ids.filter((id) => id !== personId));

  return (
    <div className="flex h-full flex-col rounded-lg border border-slate-800 bg-slate-900/40 p-3">
      <div className="mb-2 flex items-baseline gap-2">
        <h3 className="text-sm font-semibold text-slate-100">Live preview</h3>
        {personIds.length > 1 && (
          <span className="text-[11px] text-slate-500">comparing {personIds.length}</span>
        )}
        {personIds.length > 0 && (
          <button
            type="button"
            onClick={() => setPersonIds([])}
            className="ml-auto text-[11px] text-slate-500 hover:text-slate-300"
          >
            Clear all
          </button>
        )}
      </div>
      <input
        value={search}
        onChange={(e) => setSearch(e.target.value)}
        placeholder={personIds.length ? "Add another player…" : "Search a player to preview…"}
        className="mb-2 rounded-md border border-slate-700 bg-slate-950 px-2 py-1 text-xs text-slate-100 placeholder-slate-600"
      />
      {search.trim().length > 1 && (
        <div className="mb-2 max-h-40 overflow-y-auto rounded-md border border-slate-800">
          {(results.data ?? []).map((r) => {
            const already = personIds.includes(r.person_id);
            return (
              <button
                key={r.person_id}
                type="button"
                disabled={already}
                onClick={() => add(r.person_id)}
                className="block w-full px-2 py-1 text-left text-xs text-slate-200 hover:bg-slate-800/60 disabled:cursor-default disabled:text-slate-600 disabled:hover:bg-transparent"
              >
                {r.last_name}, {r.first_name}{" "}
                <span className="text-slate-500">
                  {r.position ?? ""} · {r.first_year}–{r.active ? "Present" : r.last_year}
                </span>
                {already && <span className="ml-1 text-[10px] text-slate-600">(added)</span>}
              </button>
            );
          })}
        </div>
      )}
      <div className="min-h-0 flex-1 space-y-4 overflow-y-auto">
        {personIds.length === 0 ? (
          <p className="text-xs text-slate-500">
            Pick a player to see live computed ratings. Add more to compare them.
          </p>
        ) : (
          personIds.map((id) => (
            <PreviewCard key={id} personId={id} profile={profile} onRemove={() => remove(id)} />
          ))
        )}
      </div>
    </div>
  );
}

/// One player's preview card. Each card owns its own season selection and queries, so changing one
/// player's season never recomputes another's.
function PreviewCard({
  personId,
  profile,
  onRemove,
}: {
  personId: number;
  profile: FormulaProfileDto;
  onRemove: () => void;
}) {
  const [sel, setSel] = useState<string>("");
  const year = /^\d+$/.test(sel) ? Number(sel) : null;
  const isLegends = sel === LEGENDS;

  const person = useQuery({
    queryKey: ["formula_preview_person", personId],
    queryFn: () => call("historical_player", { personId }),
  });
  // Default to the person's newest season.
  useEffect(() => {
    if (person.data && person.data.seasons.length > 0) {
      setSel((s) => s || String(person.data!.seasons[0].season_year));
    }
  }, [person.data]);

  const ratings = useQuery({
    queryKey: ["formula_preview_ratings", personId, year, profile],
    queryFn: () => call("compute_ratings", { personId, seasonYear: year!, profile }),
    enabled: year != null,
    placeholderData: keepPreviousData,
  });
  // The Legends composite for the working profile — each attribute's three best seasons averaged,
  // with Overall re-derived from those composites. Computed live, never read from the rating cache,
  // so it tracks the weights being edited rather than the last saved profile.
  const legendsPreview = useQuery({
    queryKey: ["formula_preview_legends", personId, profile],
    queryFn: () => call("compute_legends_preview", { personId, profile }),
    enabled: isLegends,
    placeholderData: keepPreviousData,
  });

  const view = isLegends ? legendsPreview : ratings;
  const fullName = person.data
    ? `${person.data.first_name} ${person.data.last_name}`.trim()
    : `Player #${personId}`;
  const seasonLabel = person.data?.seasons.find((s) => s.season_year === year)?.season_label ?? null;

  const removeButton = (
    <button
      type="button"
      onClick={onRemove}
      title="Remove from the comparison"
      className="rounded-full px-1 text-slate-500 hover:bg-slate-800 hover:text-rose-400"
    >
      ✕
    </button>
  );

  return (
    <div className="space-y-2">
      <select
        value={sel}
        onChange={(e) => setSel(e.target.value)}
        className="w-full rounded-md border border-slate-700 bg-slate-950 px-2 py-1 text-xs text-slate-100"
      >
        <optgroup label="Composite">
          <option value={LEGENDS}>Legends</option>
        </optgroup>
        <optgroup label="Seasons">
          {(person.data?.seasons ?? []).map((s) => (
            <option key={s.season_year} value={s.season_year}>
              {s.season_label}
            </option>
          ))}
        </optgroup>
      </select>
      {view.isLoading ? (
        <Spinner label="Computing…" />
      ) : view.data ? (
        <ComputedRatings
          data={view.data}
          compact
          name={fullName}
          meta={isLegends ? "Legends" : seasonLabel}
          actions={removeButton}
        />
      ) : (
        <div className="flex items-center gap-2 rounded-md border border-slate-800 bg-slate-900/50 px-3 py-2">
          <span className="min-w-0 flex-1 truncate text-xs text-slate-400">
            <span className="font-semibold text-slate-200">{fullName}</span> —{" "}
            {isLegends ? "no Legends composite (no rated seasons)." : "no ratings for this season."}
          </span>
          {removeButton}
        </div>
      )}
      {isLegends && view.data && (
        <p className="text-[11px] text-slate-500">
          Each attribute is the mean of its three best seasons; Overall is the EA formula run over
          those composites. This is the rating Puck's Picks ranks on.
        </p>
      )}
    </div>
  );
}

/** Build a fhm_key/prebuilt-key → friendly label map from the catalog. */
function labelMap(cat?: FormulaCatalogDto): Map<string, string> {
  const m = new Map<string, string>();
  if (!cat) return m;
  for (const g of cat.fhm_groups) for (const a of g.attrs) m.set(a.key, a.label);
  for (const p of cat.prebuilts) m.set(p.key, p.label);
  return m;
}
