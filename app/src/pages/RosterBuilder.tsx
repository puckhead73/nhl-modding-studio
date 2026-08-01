import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import { useRosterCart, type CartItem } from "../lib/rosterCart";
import { Button, Card, EmptyState, Pill } from "../components/primitives";
import { POS_BADGE } from "../lib/position";
import type { ImportItemDto } from "../bindings/ImportItemDto";
import type { ChangeSetPreviewDto } from "../bindings/ChangeSetPreviewDto";
import type { ImportResultDto as ImportResult } from "../bindings/ImportResultDto";
import type { ImportItemResultDto } from "../bindings/ImportItemResultDto";
import type { FindingDto } from "../bindings/FindingDto";

/// Roster Builder (Phase 5.5) — the cart of corpus person-seasons queued for import into the open EA
/// project. Each row picks a target team + Match/Create + jersey; Preview runs a non-mutating dry-run
/// (per-item outcomes, capacity delta, audit, blockers, dedup), and Commit applies the whole cart as
/// one undo entry. Requires an open project (AppShell) and an open corpus (CorpusGate).
export function RosterBuilderPage() {
  const { project } = useProject();
  const cart = useRosterCart();
  const qc = useQueryClient();
  const projectId = project?.project_id ?? null;

  const [preview, setPreview] = useState<ChangeSetPreviewDto | null>(null);
  const [result, setResult] = useState<ImportResult | null>(null);

  const teams = useQuery({
    queryKey: ["list_teams", projectId],
    queryFn: () => call("list_teams", { projectId: projectId! }),
    enabled: projectId != null,
  });

  // The cart lowers to the backend item shape; a row missing a target team can't be sent.
  const ready = cart.items.length > 0 && cart.items.every((i) => i.targetTeamId != null);
  const toDto = (i: CartItem): ImportItemDto => ({
    person_id: i.personId,
    season_year: i.seasonYear,
    target_team_id: i.targetTeamId!,
    match_ea_player_id: i.matchEaPlayerId,
    jersey: i.jersey,
  });

  const doPreview = useMutation({
    mutationFn: () => call("import_preview", { projectId: projectId!, items: cart.items.map(toDto) }),
    onSuccess: (p) => {
      setPreview(p);
      setResult(null);
    },
  });

  const doCommit = useMutation({
    mutationFn: () => call("import_commit", { projectId: projectId!, items: cart.items.map(toDto) }),
    onSuccess: (r) => {
      setResult(r);
      setPreview(null);
      cart.clear();
      // The roster changed — refresh the Game-Database views + project summary.
      qc.invalidateQueries({ queryKey: ["list_players", projectId] });
      qc.invalidateQueries({ queryKey: ["list_teams", projectId] });
      qc.invalidateQueries({ queryKey: ["project_summary", projectId] });
    },
  });

  // Any cart edit invalidates a stale preview (commit must reflect the current cart).
  const editItem = (id: string, patch: Partial<CartItem>) => {
    cart.update(id, patch);
    setPreview(null);
  };
  const removeItem = (id: string) => {
    cart.remove(id);
    setPreview(null);
  };

  if (projectId == null) {
    return <EmptyState title="No project open" hint="Open an EA project to build a roster into it." />;
  }

  return (
    <div className="max-w-5xl space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-lg font-semibold text-slate-100">Roster Builder</h2>
          <p className="text-xs text-slate-400">
            Import historical player-seasons into the open project. Send players here from the
            Historical browse, choose a target team, then Preview → Commit.
          </p>
        </div>
        {cart.items.length > 0 && (
          <Button variant="ghost" onClick={() => { cart.clear(); setPreview(null); }}>
            Clear cart
          </Button>
        )}
      </div>

      {cart.items.length === 0 ? (
        <EmptyState
          title="The cart is empty"
          hint="Open a Historical player and use “Send to Builder” on a season row to queue an import."
        />
      ) : (
        <Card
          title={`Cart — ${cart.items.length} ${cart.items.length === 1 ? "player" : "players"}`}
          actions={
            <div className="flex items-center gap-2">
              <Button
                variant="secondary"
                disabled={!ready || doPreview.isPending}
                onClick={() => doPreview.mutate()}
              >
                {doPreview.isPending ? "Previewing…" : "Preview"}
              </Button>
              <Button
                disabled={!preview || preview.blocked > 0 || preview.capacity_blocked || doCommit.isPending}
                onClick={() => doCommit.mutate()}
                title={
                  preview && preview.blocked > 0
                    ? "Resolve blockers before committing"
                    : preview && preview.capacity_blocked
                      ? "The roster would exceed its capacity — free space or split the import"
                      : undefined
                }
              >
                {doCommit.isPending ? "Committing…" : "Commit"}
              </Button>
            </div>
          }
        >
          <div className="overflow-x-auto">
            <table className="w-full border-collapse text-sm">
              <thead>
                <tr className="text-left text-[11px] font-semibold uppercase tracking-wider text-slate-500">
                  <th className="px-2 py-1">Player</th>
                  <th className="px-2 py-1">Season</th>
                  <th className="px-2 py-1">Target team</th>
                  <th className="px-2 py-1">Identity</th>
                  <th className="px-2 py-1">Jersey</th>
                  <th className="px-2 py-1"></th>
                </tr>
              </thead>
              <tbody>
                {cart.items.map((it) => (
                  <CartRow
                    key={it.id}
                    item={it}
                    projectId={projectId}
                    teams={teams.data ?? []}
                    onChange={(patch) => editItem(it.id, patch)}
                    onRemove={() => removeItem(it.id)}
                  />
                ))}
              </tbody>
            </table>
          </div>
          {!ready && (
            <p className="mt-2 text-xs text-amber-400">Every row needs a target team before you can preview.</p>
          )}
          {doPreview.isError && <p className="mt-2 text-xs text-rose-400">{String(doPreview.error)}</p>}
          {doCommit.isError && <p className="mt-2 text-xs text-rose-400">{String(doCommit.error)}</p>}
        </Card>
      )}

      {preview && <PreviewPanel preview={preview} teams={teams.data ?? []} />}
      {result && <CommitResult result={result} />}
    </div>
  );
}

/** One editable cart row. Two ways to place the player, unified in one control (Phase 5.5b):
 *  identity **matches** (same person, name+DOB — fetched lazily via the corpus↔project bridge) and
 *  manual **slot fills** (replace any current occupant of the chosen target team). Both resolve to a
 *  `wBIz` (update); "Add as a new player" creates. */
function CartRow({
  item,
  projectId,
  teams,
  onChange,
  onRemove,
}: {
  item: CartItem;
  projectId: number;
  teams: { team_id: number; name: string }[];
  onChange: (patch: Partial<CartItem>) => void;
  onRemove: () => void;
}) {
  const matches = useQuery({
    queryKey: ["match_ea_player", projectId, item.personId],
    queryFn: () => call("match_ea_player", { projectId, personId: item.personId }),
  });
  // The whole roster (cached, shared key) — filtered to the chosen team for the slot-fill options.
  const roster = useQuery({
    queryKey: ["list_players", projectId],
    queryFn: () => call("list_players", { projectId }),
  });

  const targetTeamName = teams.find((t) => t.team_id === item.targetTeamId)?.name;
  const matchIds = new Set((matches.data ?? []).map((c) => c.ea_player_id));
  // Current occupants of the target team, minus anyone already offered as a same-person match.
  const occupants =
    item.targetTeamId == null
      ? []
      : (roster.data ?? [])
          .filter((p) => p.team_id === item.targetTeamId && !matchIds.has(p.player_id))
          .sort((a, b) => `${a.last_name}${a.first_name}`.localeCompare(`${b.last_name}${b.first_name}`));

  const selectCls =
    "rounded-md border border-slate-700 bg-slate-950 px-2 py-1 text-xs text-slate-100 focus:border-sky-600 focus:outline-none";

  return (
    <tr className="border-t border-slate-800/60 align-middle">
      <td className="px-2 py-1.5">
        <div className="flex items-center gap-1.5">
          {item.position && (
            <span
              className={
                "inline-flex items-center rounded px-1 py-0.5 text-[9px] font-semibold " +
                (POS_BADGE[item.position] ?? "bg-slate-800 text-slate-300")
              }
            >
              {item.position}
            </span>
          )}
          <span className="text-slate-100">{item.personName}</span>
        </div>
      </td>
      <td className="px-2 py-1.5 tabular-nums text-slate-300">{item.seasonLabel}</td>
      <td className="px-2 py-1.5">
        <select
          className={selectCls}
          value={item.targetTeamId ?? ""}
          onChange={(e) =>
            // Changing team invalidates a slot-fill chosen against the old team.
            onChange({ targetTeamId: e.target.value ? Number(e.target.value) : null, matchEaPlayerId: null })
          }
        >
          <option value="">Select team…</option>
          {teams.map((t) => (
            <option key={t.team_id} value={t.team_id}>
              {t.name}
            </option>
          ))}
        </select>
      </td>
      <td className="px-2 py-1.5">
        {matches.isLoading ? (
          <span className="text-xs text-slate-500">…</span>
        ) : (
          <select
            className={selectCls}
            value={item.matchEaPlayerId ?? ""}
            onChange={(e) =>
              onChange({ matchEaPlayerId: e.target.value ? Number(e.target.value) : null })
            }
          >
            <option value="">Add as a new player</option>
            {(matches.data ?? []).length > 0 && (
              <optgroup label="Same person (suggested)">
                {(matches.data ?? []).map((c) => (
                  <option key={c.ea_player_id} value={c.ea_player_id}>
                    Update {c.full_name}
                    {c.team_name ? ` (${c.team_name})` : ""}
                    {c.confidence === "name_only" ? " — name only" : ""}
                  </option>
                ))}
              </optgroup>
            )}
            {occupants.length > 0 && (
              <optgroup label={`Replace on ${targetTeamName ?? "team"}`}>
                {occupants.map((p) => (
                  <option key={p.player_id} value={p.player_id}>
                    {p.position_label ? `${p.position_label} · ` : ""}
                    {p.first_name} {p.last_name}
                    {p.jersey_number != null ? ` #${p.jersey_number}` : ""}
                  </option>
                ))}
              </optgroup>
            )}
          </select>
        )}
      </td>
      <td className="px-2 py-1.5">
        <input
          type="number"
          className={selectCls + " w-16"}
          value={item.jersey ?? ""}
          placeholder="—"
          onChange={(e) => onChange({ jersey: e.target.value ? Number(e.target.value) : null })}
        />
      </td>
      <td className="px-2 py-1.5 text-right">
        <button className="text-xs text-slate-500 hover:text-rose-400" onClick={onRemove}>
          Remove
        </button>
      </td>
    </tr>
  );
}

const OUTCOME_TONE: Record<string, "ok" | "info" | "error" | "warn"> = {
  created: "ok",
  matched: "info",
  person_missing: "error",
  no_such_player: "error",
  no_bio_table: "error",
};
const OUTCOME_LABEL: Record<string, string> = {
  created: "Create",
  matched: "Update",
  person_missing: "Person not found",
  no_such_player: "Match id missing",
  no_bio_table: "No bio table",
};

/** The dry-run panel: the roll-up (player delta, blockers), the per-item outcomes, and the audit. */
function PreviewPanel({
  preview,
  teams,
}: {
  preview: ChangeSetPreviewDto;
  teams: { team_id: number; name: string }[];
}) {
  const teamName = (id: number) => teams.find((t) => t.team_id === id)?.name ?? `Team #${id}`;
  const net = preview.players_after - preview.players_before;
  return (
    <Card
      title="Preview"
      subtitle="A dry-run — nothing is written until you Commit. Commit produces byte-identical edits."
      actions={
        <div className="flex items-center gap-2">
          <Pill tone={net > 0 ? "ok" : "neutral"}>
            {net > 0 ? `+${net}` : net} players
          </Pill>
          {preview.blocked > 0 ? (
            <Pill tone="error">{preview.blocked} blocked</Pill>
          ) : (
            <Pill tone="ok">no blockers</Pill>
          )}
        </div>
      }
    >
      <ul className="space-y-1">
        {preview.items.map((it, i) => (
          <PreviewRow key={i} it={it} teamName={teamName} />
        ))}
      </ul>

      {preview.skipped_fields.length > 0 && (
        <p className="mt-3 text-xs text-amber-400">
          Skipped (absent or out of range on this title):{" "}
          <span className="text-slate-400">{preview.skipped_fields.join(", ")}</span>
        </p>
      )}

      <CapacityBars capacity={preview.capacity} blocked={preview.capacity_blocked} />
      <AuditSummary audit={preview.audit} />
    </Card>
  );
}

/** The S4 storage gate — a bar per roster table the cart adds to, current+adding vs the vanilla
 *  ceiling. Only tables the cart touches are shown; an overflowing one is flagged (Commit is blocked).
 *  Unenforced ceilings (unknown title on a packed file) render without a blocker. */
function CapacityBars({
  capacity,
  blocked,
}: {
  capacity: ChangeSetPreviewDto["capacity"];
  blocked: boolean;
}) {
  const rows = capacity.filter((c) => c.adding > 0);
  if (rows.length === 0) return null;
  return (
    <div className="mt-3">
      <div className="mb-1 flex items-center gap-2 text-[11px] uppercase tracking-wide text-slate-500">
        Roster capacity
        {blocked && <Pill tone="error">over capacity</Pill>}
      </div>
      <div className="space-y-1">
        {rows.map((c) => {
          const after = c.current + c.adding;
          const pct = c.ceiling > 0 ? Math.min(100, (after / c.ceiling) * 100) : 0;
          const basePct = c.ceiling > 0 ? Math.min(100, (c.current / c.ceiling) * 100) : 0;
          return (
            <div key={c.table} className="flex items-center gap-2 text-xs">
              <span className="w-32 shrink-0 text-slate-400" title={c.table}>
                {c.label}
              </span>
              <div className="relative h-2 flex-1 overflow-hidden rounded-full bg-slate-800">
                <div
                  className={"absolute inset-y-0 left-0 " + (c.over ? "bg-rose-500/80" : "bg-sky-600/70")}
                  style={{ width: `${pct}%` }}
                />
                <div className="absolute inset-y-0 left-0 bg-slate-500/50" style={{ width: `${basePct}%` }} />
              </div>
              <span
                className={
                  "w-28 shrink-0 text-right tabular-nums " + (c.over ? "text-rose-300" : "text-slate-500")
                }
              >
                {after.toLocaleString()} / {c.ceiling.toLocaleString()}
                {!c.enforced && " *"}
              </span>
            </div>
          );
        })}
      </div>
      {rows.some((c) => !c.enforced) && (
        <p className="mt-1 text-[11px] text-slate-500">* ceiling unknown for this title — not enforced.</p>
      )}
    </div>
  );
}

function PreviewRow({
  it,
  teamName,
}: {
  it: ImportItemResultDto;
  teamName: (id: number) => string;
}) {
  const tone = OUTCOME_TONE[it.outcome] ?? "neutral";
  return (
    <li className="flex flex-wrap items-center gap-2 rounded-md border border-slate-800/60 bg-slate-900/40 px-3 py-1.5 text-sm">
      <Pill tone={tone}>{OUTCOME_LABEL[it.outcome] ?? it.outcome}</Pill>
      <span className="text-slate-100">{it.full_name || `Person #${it.person_id}`}</span>
      <span className="text-xs text-slate-500">→ {teamName(it.target_team_id)}</span>
      {it.already_imported && (
        <span title="Already imported to this target (advisory)">
          <Pill tone="warn">already imported</Pill>
        </span>
      )}
      {it.skipped_fields.length > 0 && (
        <span className="text-[11px] text-amber-400/80">skipped {it.skipped_fields.length}</span>
      )}
    </li>
  );
}

/** Reuses the Dashboard finding vocabulary at a glance — the four audit predicates over the previewed
 *  state. A clean report is the common case. */
function AuditSummary({ audit }: { audit: ChangeSetPreviewDto["audit"] }) {
  if (audit.clean && audit.findings.length === 0) {
    return <p className="mt-3 text-xs text-emerald-400">Validation clean — no findings on the previewed state.</p>;
  }
  return (
    <div className="mt-3 space-y-1">
      <div className="flex items-center gap-2 text-xs">
        {audit.error_count > 0 && <Pill tone="error">{audit.error_count} errors</Pill>}
        {audit.warning_count > 0 && <Pill tone="warn">{audit.warning_count} warnings</Pill>}
        {audit.info_count > 0 && <Pill tone="info">{audit.info_count} info</Pill>}
      </div>
      <ul className="space-y-0.5">
        {audit.findings.slice(0, 8).map((f: FindingDto, i: number) => (
          <li key={i} className="text-xs text-slate-400">
            <span className="text-slate-300">{f.title}</span>
            {f.location ? <span className="text-slate-500"> — {f.location}</span> : null}
          </li>
        ))}
      </ul>
    </div>
  );
}

function CommitResult({ result }: { result: ImportResult }) {
  const created = result.items.filter((i) => i.outcome === "created").length;
  const matched = result.items.filter((i) => i.outcome === "matched").length;
  return (
    <Card title="Imported">
      <div className="flex flex-wrap items-center gap-2 text-sm">
        <Pill tone="ok">{created} created</Pill>
        {matched > 0 && <Pill tone="info">{matched} updated</Pill>}
        <span className="text-xs text-slate-400">
          {result.outcome.edits} cell edits · {result.ledgered} ledgered
        </span>
      </div>
      <p className="mt-2 text-xs text-slate-500">
        The import is a single undo entry — reverse it from the pending-edits history. Save the project
        to write it to disk.
      </p>
    </Card>
  );
}
