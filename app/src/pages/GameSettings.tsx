import { assetRoot } from "../lib/assetRoot";
import { GamePlatformPill } from "../components/GamePlatformPill";
import { useEffect, useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import { Card, EmptyState, ErrorBanner, Pill, type Tone } from "../components/primitives";
import type { SettingRowDto } from "../bindings/SettingRowDto";
import type { SettingsCatalogueDto } from "../bindings/SettingsCatalogueDto";
import type { StoredDefaultDto } from "../bindings/StoredDefaultDto";
import { StoredDefaultEditor } from "../components/StoredDefaultEditor";

/// Phase 1.2 (D2) — the mode-setup settings catalogue, read-only. Each row is one catalogue entry
/// (identity = the legacy id): its stored global default, its per-screen (context) defaults, the
/// choice codes and the labels they map to, and whether the row is in the stored-default editing
/// contract. Every name that is not the title's own data is marked (lineage / inferred). Nothing
/// on this page says what the game offers or applies; that is a runtime question.
export function GameSettingsPage() {
  const { project, install, iso } = useProject();
  const projectId = project!.project_id;
  const installRoot = assetRoot(install) ?? iso?.root ?? null;
  const q = useQuery({
    queryKey: ["settings-catalogue", projectId, installRoot],
    queryFn: () => call("settings_catalogue", { projectId, installRoot }),
  });
  const [scope, setScope] = useState<"contract" | "all">("contract");
  const [search, setSearch] = useState("");
  const [selected, setSelected] = useState<bigint | null>(null);
  // Selection belongs to the project + source it was made on; a switch drops it.
  useEffect(() => setSelected(null), [projectId, installRoot]);

  const cat = q.data?.catalogue ?? null;
  const rows = useMemo(() => {
    if (!cat) return [];
    const needle = search.trim().toLowerCase();
    return cat.settings
      .filter((s) => scope === "all" || s.eligibility !== "out-of-scope")
      .filter((s) => needle === "" || `${s.legacy_id} ${s.name} ${s.label_text ?? ""} ${s.role ?? ""} ${s.label_key ?? ""}`.toLowerCase().includes(needle));
  }, [cat, scope, search]);

  if (q.isLoading) return <div className="p-4 text-sm text-slate-400">Reading the settings catalogue…</div>;
  if (q.error) return <ErrorBanner title="Settings catalogue" message={String(q.error)} />;
  if (!cat) {
    return (
      <div className="p-4">
        <EmptyState
          title="No settings catalogue for this target"
          hint={
            <span data-testid="settings-blocker">
              {q.data?.blocker ?? "unknown reason"}
              {q.data?.blocker_kind && <span className="ml-2 text-slate-500">({q.data.blocker_kind})</span>}
            </span>
          }
        />
      </div>
    );
  }
  const current = selected === null ? null : (cat.settings.find((s) => s.legacy_id === selected) ?? null);
  return (
    <div className="flex h-full flex-col gap-3 p-4">
      <SourceSummary cat={cat} />
      <div className="flex items-center gap-3 text-xs">
        <label className="flex items-center gap-1 text-slate-400">
          <input type="radio" name="scope" checked={scope === "contract"} onChange={() => setScope("contract")} />
          league &amp; competition settings
        </label>
        <label className="flex items-center gap-1 text-slate-400">
          <input type="radio" name="scope" checked={scope === "all"} onChange={() => setScope("all")} />
          all {cat.settings.length} settings
        </label>
        <input
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          placeholder="Search setting / role / id…"
          className="ml-auto w-56 rounded border border-slate-700 bg-slate-950 px-2 py-1 text-xs text-slate-100 placeholder-slate-600 focus:border-sky-600 focus:outline-none"
        />
        {cat.absent_eligible.length > 0 && (
          <span className="text-slate-500" title={`Contract ids this title's catalogue does not carry: ${cat.absent_eligible.map(String).join(", ")}`}>
            {cat.absent_eligible.length} contract ids absent on this title
          </span>
        )}
      </div>
      <div className="grid min-h-0 flex-1 grid-cols-[minmax(0,3fr)_minmax(0,2fr)] gap-3">
        <div className="min-h-0 overflow-auto rounded-lg border border-slate-800">
          <table className="w-full text-left text-xs">
            <thead className="sticky top-0 bg-slate-900 text-slate-500">
              <tr>
                <th className="px-2 py-1">id</th>
                <th className="px-2 py-1">setting</th>
                <th className="px-2 py-1">role</th>
                <th className="px-2 py-1">range</th>
                <th className="px-2 py-1">global default</th>
                <th className="px-2 py-1">screens</th>
                <th className="px-2 py-1">contract</th>
              </tr>
            </thead>
            <tbody>
              {rows.map((s) => (
                <tr
                  key={String(s.legacy_id)}
                  data-testid={`setting-row-${s.legacy_id}`}
                  onClick={() => setSelected(s.legacy_id)}
                  className={"cursor-pointer border-t border-slate-800 hover:bg-slate-800/40 " + (selected === s.legacy_id ? "bg-slate-800/60" : "")}
                >
                  <td className="px-2 py-1 font-mono text-slate-500">{String(s.legacy_id)}</td>
                  <td className="px-2 py-1">
                    <span className={s.label_text ? "text-slate-100" : "font-mono text-amber-200/90"}>{s.label_text ?? s.name}</span>
                    {s.label_text && <span className="ml-1 font-mono text-[10px] text-slate-500">{s.name}</span>}
                  </td>
                  <td className="px-2 py-1 font-mono text-[10px] text-slate-400">{s.role ?? ""}</td>
                  <td className="px-2 py-1 font-mono text-slate-400">{s.min !== null && s.max !== null ? `${s.min}..${s.max}` : "—"}</td>
                  <td className="px-2 py-1">
                    <DefaultCell d={s.global} source={s.label_source} />
                  </td>
                  <td className="px-2 py-1 tabular-nums text-slate-400">{s.context_defaults.length}</td>
                  <td className="px-2 py-1">
                    <EligibilityPill s={s} />
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
          {rows.length === 0 && <div className="p-4 text-xs text-slate-500">No setting matches.</div>}
        </div>
        <div className="min-h-0 overflow-auto">
          {current ? <SettingDetail s={current} cat={cat} projectId={projectId} installRoot={installRoot} /> : <EmptyState title="Select a setting" hint="The stored defaults, screens and choice codes of the selected row appear here." />}
        </div>
      </div>
      <Disclosures cat={cat} />
    </div>
  );
}

function SourceSummary({ cat }: { cat: SettingsCatalogueDto }) {
  const src = cat.source;
  const what =
    src.kind === "database"
      ? `Read from the open database's five catalogue tables (${src.copy ?? ""}): ${src.tables.map((t) => `${t.name} ${t.rows}`).join(" · ")}.`
      : `Read from ${src.members.length} setting_csv members of ${src.origin ?? ""} (schema ${src.variant ?? ""}).`;
  return (
    <div className="flex flex-wrap items-center gap-2 text-xs text-slate-400" data-testid="settings-source">
      <GamePlatformPill gameId={cat.game} platform={cat.platform} />
      <span>{what}</span>
      {cat.dictionary ? <Pill tone="ok">labels via dictionary</Pill> : <Pill tone="neutral">no dictionary — labels shown as keys</Pill>}
      {src.kind === "database" ? <Pill tone="info">stored defaults editable (one cell at a time)</Pill> : <Pill tone="info">stored defaults editable (pending until a copied-target save)</Pill>}
    </div>
  );
}

function sourceTone(source: string): Tone {
  return source === "explicit" ? "ok" : source === "none" ? "neutral" : "warn";
}

function DefaultCell({ d, source }: { d: StoredDefaultDto | null; source: string }) {
  if (!d) return <span className="text-slate-500">—</span>;
  if (d.code === null) return <Pill tone="neutral">null (empty cell)</Pill>;
  return (
    <span className="inline-flex items-center gap-1">
      <span className="font-mono text-slate-200">{String(d.code)}</span>
      {d.label && (
        <span className="text-slate-300" title={`label ${source}`}>
          {d.label}
        </span>
      )}
      {d.label && source !== "explicit" && <Pill tone={sourceTone(source)}>{source}</Pill>}
      {d.paired && (
        <Pill tone={d.consistent ? "info" : "error"} >
          {d.consistent ? "stored twice, equal" : `stored twice, differ: ${d.values.map((v) => (v === null ? "null" : String(v))).join(" / ")}`}
        </Pill>
      )}
    </span>
  );
}

function EligibilityPill({ s }: { s: SettingRowDto }) {
  if (s.eligibility === "eligible") return <Pill tone="ok">in contract</Pill>;
  if (s.eligibility === "excluded")
    return (
      <span title={s.eligibility_reason ?? ""}>
        <Pill tone="warn">excluded</Pill>
      </span>
    );
  return <span className="text-slate-600">—</span>;
}

function SettingDetail({ s, cat, projectId, installRoot }: { s: SettingRowDto; cat: SettingsCatalogueDto; projectId: number; installRoot: string | null }) {
  const contextName = (id: bigint) => cat.contexts.find((c) => c.id === id);
  return (
    <Card
      title={
        <span>
          {s.label_text ?? s.name} <span className="font-mono text-xs text-slate-500">#{String(s.legacy_id)}</span>
        </span>
      }
      subtitle={
        <span>
          {s.role && <span className="font-mono">{s.role} · </span>}
          {s.setting_id !== null && (
            <span>
              setting identity {String(s.setting_id)}
              {s.siblings.length > 0 && <span> (shared with rows {s.siblings.map(String).join(", ")} — informational, not targets)</span>} ·{" "}
            </span>
          )}
          {s.min !== null && s.max !== null ? `codes ${s.min}..${s.max}` : "no numeric range"}
          {s.size !== null ? ` · size ${s.size}` : ""}
        </span>
      }
    >
      <div className="space-y-3 text-xs" data-testid="setting-detail">
        {s.eligibility === "excluded" && <div className="text-amber-300/90">Not in the stored-default contract: {s.eligibility_reason}</div>}
        {s.eligibility === "eligible" && <StoredDefaultEditor projectId={projectId} setting={s} installRoot={installRoot} />}
        {s.eligibility === "eligible" && cat.source.kind !== "database" && (
          <div className="text-slate-500">On this title the edit rewrites one cell of a CSV text member inside the archive; it stays pending until the next copied-target save re-compresses and places the member.</div>
        )}
        <section>
          <div className="mb-1 text-[11px] uppercase tracking-wide text-slate-500">Global stored default</div>
          <DefaultCell d={s.global} source={s.label_source} />
          {s.global && (
            <ul className="mt-1 font-mono text-[10px] text-slate-500">
              {s.global.cells.map((c) => (
                <li key={c}>{c}</li>
              ))}
            </ul>
          )}
        </section>
        <section>
          <div className="mb-1 text-[11px] uppercase tracking-wide text-slate-500">
            Stored defaults per screen ({s.context_defaults.length})
          </div>
          {s.context_defaults.length === 0 ? (
            <div className="text-slate-500">No per-screen default is stored for this setting; only the global default exists. Whether a screen uses it is not established.</div>
          ) : (
            <table className="w-full text-left">
              <thead className="text-slate-500">
                <tr>
                  <th className="py-0.5 pr-2">screen</th>
                  <th className="py-0.5 pr-2">stored</th>
                  <th className="py-0.5 pr-2">flags</th>
                </tr>
              </thead>
              <tbody>
                {s.context_defaults.map((d, i) => {
                  const c = contextName(d.context);
                  return (
                    <tr key={i} className="border-t border-slate-800 align-top" data-testid={`context-default-${d.context}`}>
                      <td className="py-0.5 pr-2">
                        <span className="font-mono text-slate-400">{String(d.context)}</span>{" "}
                        {d.context_name ? (
                          <span className={c?.name_source === "lineage" ? "text-amber-200/90" : "text-slate-200"} title={c?.name_source === "lineage" ? "name by lineage from another title's numbering, not stored on this title" : "name stored on this title"}>
                            {d.context_name}
                          </span>
                        ) : (
                          <span className="text-slate-500">unnamed</span>
                        )}
                        {c?.name_source === "lineage" && <Pill tone="warn">lineage</Pill>}
                        <div className="font-mono text-[10px] text-slate-600">{d.stored.cells.join("; ")}</div>
                      </td>
                      <td className="py-0.5 pr-2">
                        <DefaultCell d={d.stored} source={d.choices ? "explicit" : s.label_source} />
                        {d.domain_note && <div className="text-amber-300/80">{d.domain_note}</div>}
                        {d.anomaly && <div className="text-rose-300/90">{d.anomaly}</div>}
                      </td>
                      <td className="py-0.5 pr-2 text-slate-500">
                        {d.enabled === false && <Pill tone="warn">is_enabled 0</Pill>}
                        {d.visible === false && <Pill tone="warn">is_visible 0</Pill>}
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          )}
        </section>
        <section>
          <div className="mb-1 text-[11px] uppercase tracking-wide text-slate-500">
            Choice codes {s.choices.length > 0 && <Pill tone={sourceTone(s.label_source)}>labels {s.label_source}</Pill>}
          </div>
          {s.choices.length === 0 ? (
            <div className="text-slate-500">No label array is bound to this row's codes; codes are shown raw.</div>
          ) : (
            <ul className="grid grid-cols-2 gap-x-3 font-mono">
              {s.choices.map((c) => (
                <li key={String(c.code)}>
                  <span className="text-slate-400">{String(c.code)}</span> <span className="text-slate-200">{c.text ?? c.key}</span>
                  {c.text && <span className="text-[10px] text-slate-600"> {c.key}</span>}
                </li>
              ))}
            </ul>
          )}
        </section>
        {(s.groups.length > 0 || s.dependents.length > 0) && (
          <section className="text-slate-500">
            {s.groups.length > 0 && <div>groups: {s.groups.map((g) => cat.groups.find((x) => x.id === g)?.name ?? String(g)).join(", ")}</div>}
            {s.dependents.length > 0 && <div>re-evaluates when changed: {s.dependents.map(String).join(", ")} (meaning of the dependency unknown)</div>}
          </section>
        )}
        {s.anomalies.length > 0 && (
          <section>
            <div className="mb-1 text-[11px] uppercase tracking-wide text-slate-500">Row anomalies (stock, preserved)</div>
            <ul className="list-disc pl-4 text-amber-300/80">
              {s.anomalies.map((a) => (
                <li key={a}>{a}</li>
              ))}
            </ul>
          </section>
        )}
      </div>
    </Card>
  );
}

function Disclosures({ cat }: { cat: SettingsCatalogueDto }) {
  const [open, setOpen] = useState(false);
  return (
    <div className="text-xs">
      <button type="button" className="text-sky-400 underline-offset-2 hover:underline" onClick={() => setOpen(!open)} data-testid="settings-disclosures">
        {open ? "hide" : "show"} evidence notes ({cat.notes.length}) and source anomalies ({cat.anomalies.length})
      </button>
      {open && (
        <div className="mt-2 grid grid-cols-2 gap-3">
          <ul className="list-disc pl-4 text-slate-400">
            {cat.notes.map((n) => (
              <li key={n}>{n}</li>
            ))}
          </ul>
          <ul className="list-disc pl-4 text-amber-300/70">
            {cat.anomalies.length === 0 ? <li>none</li> : cat.anomalies.map((a) => <li key={a}>{a}</li>)}
          </ul>
        </div>
      )}
    </div>
  );
}
