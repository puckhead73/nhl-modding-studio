// Phase 2.6 (22 Sep 2026) — the read-only "Kit textures" card on the Uniforms tab.
//
// Shows what the backend measured for the selected kit: the title's kit-file contract, how each
// component directory of the rendering tree read, every required component (jersey / pants / each
// sock file the title's grammar requires) with present / missing / unknown evidence and a reason,
// the completeness verdict, the shipped spare candidates judged by the same rule, and why
// re-pointing a row is not offered.
//
// Replacement (Teams 2.6, 24 Sep 2026): a **present** component offers Replace… — a loose file
// through the reviewed loose-texture route (`onReplaceLoose`, the Uniforms tab's dialog), a render-
// archive member (PS3 NHL 12 / 13, `chunkzip`-wrapped per research R76) through the archive-member
// route (preview → Apply as a pending replacement → the next copied-target save). The NHL 11 / 12
// sock *pair* is not offered file-by-file: both files are one kit and the per-pair mirror class is
// not measured, so a single-file write could leave the pair inconsistent — the card says so.
//
// Result ownership: the query is keyed on project + team + install root + platform, and a payload
// is only rendered when its `team_id` is the team the card is showing — a late answer for a team
// the user has moved away from is never painted as this team's evidence.

import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { call } from "../lib/client";
import type { AssetRootsArgsDto } from "../bindings/AssetRootsArgsDto";
import type { KitComponentDto } from "../bindings/KitComponentDto";
import type { KitDto } from "../bindings/KitDto";
import type { SpareKitDto } from "../bindings/SpareKitDto";
import type { TeamKitsDto } from "../bindings/TeamKitsDto";
import { ArchiveMemberThumbnail, TextureThumbnail } from "./TextureReplace";
import { useArchiveMemberReplace } from "./ArchiveMemberReplace";
import { Spinner } from "./primitives";

const STATUS_TONE: Record<string, string> = {
  present: "border-emerald-700/60 bg-emerald-900/30 text-emerald-200",
  missing: "border-rose-700/60 bg-rose-900/30 text-rose-200",
  unknown: "border-amber-700/60 bg-amber-900/30 text-amber-200",
};

const VERDICT_TONE: Record<string, string> = {
  complete: "border-emerald-700/60 bg-emerald-900/30 text-emerald-200",
  incomplete: "border-rose-700/60 bg-rose-900/30 text-rose-200",
  unknown: "border-amber-700/60 bg-amber-900/30 text-amber-200",
};

function Badge({ tone, children, testId }: { tone: string; children: React.ReactNode; testId?: string }) {
  return (
    <span className={"rounded border px-1.5 py-0.5 text-[10px] font-semibold uppercase tracking-wide " + tone} data-testid={testId}>
      {children}
    </span>
  );
}

function count<T>(items: T[], pick: (t: T) => string, value: string): number {
  return items.filter((i) => pick(i) === value).length;
}

export function TeamKitsCard({
  projectId,
  teamId,
  rootArgs,
  installDir,
  style,
  variant,
  onReplaceLoose,
}: {
  projectId: number;
  teamId: number;
  rootArgs: AssetRootsArgsDto;
  installDir: string | null;
  /** The kit selected in the dropdown; `null` when the selection is the unparseable catch-all. */
  style: number | null;
  variant: number | null;
  /** Replace a loose component through the caller's reviewed loose-texture dialog (`relPath`, label). */
  onReplaceLoose?: (relPath: string, label: string) => void;
}) {
  const q = useQuery({
    queryKey: ["teamKits", projectId, teamId, installDir ?? "", rootArgs.platform ?? "", rootArgs.rendering_override ?? ""],
    queryFn: () => call("team_kits", { projectId, roots: rootArgs, teamId }),
    staleTime: Infinity,
    // Refresh loose/archive presence after extraction without discarding the selected kit.
    refetchOnMount: "always",
    retry: false,
  });
  const [showSpares, setShowSpares] = useState(false);
  // Archive members (PS3 NHL 12 / 13): the archive-member route, re-listing the card after an apply.
  const archiveReplace = useArchiveMemberReplace(projectId, installDir, () => void q.refetch());

  if (q.isError) {
    return (
      <section className="rounded-md border border-rose-900/60 bg-rose-950/30 px-3 py-2 text-xs text-rose-200" data-testid="kits-error">
        Kit textures could not be read: {String(q.error)}
      </section>
    );
  }
  // A payload is this team's evidence only when it says so — otherwise it is a late answer for
  // another team (or a stale cache entry) and is treated as still loading.
  const d: TeamKitsDto | undefined = q.data && q.data.team_id === teamId ? q.data : undefined;
  if (!d) return <Spinner label="Reading this team's kit textures…" />;

  const row: KitDto | undefined =
    style == null || variant == null ? undefined : d.kits.find((k) => k.style === style && k.variant === variant);
  const spare: SpareKitDto | undefined =
    row || style == null || variant == null ? undefined : d.spares.find((sp) => sp.style === style && sp.variant === variant);
  const rows = d.kits.filter((k) => !k.refusal);
  const refused = d.kits.length - rows.length;

  return (
    <section className="space-y-2 rounded-md border border-slate-800 bg-slate-950/40 px-3 py-2 text-xs" data-testid="kits-card" data-team-id={d.team_id}>
      <div className="flex flex-wrap items-center justify-between gap-2">
        <h3 className="text-sm font-semibold text-slate-100">Kit assignments</h3>
        <Badge
          tone={
            d.contract_status === "measured"
              ? "border-sky-700/60 bg-sky-900/30 text-sky-200"
              : "border-amber-700/60 bg-amber-900/30 text-amber-200"
          }
          testId="kits-contract-status"
        >
          {d.contract_status === "measured"
            ? d.sock_grammar === "two_file"
              ? "measured · two sock files per kit"
              : "measured · one sock file per kit"
            : d.contract_status === "no_selector_table"
              ? "no selector table"
              : "not measured"}
        </Badge>
      </div>
      <p className="text-slate-400" data-testid="kits-contract" title={d.contract}>
        {d.contract}
      </p>

      {d.directories.length > 0 ? (
        <p className="text-slate-500" data-testid="kits-directories">
          Rendering tree:{" "}
          {d.directories.map((dir, i) => (
            <span key={dir.part} title={dir.reason ?? undefined}>
              {i > 0 ? " · " : ""}
              <code>{dir.part}/</code>{" "}
              {dir.status === "listed" ? `${dir.files ?? 0} files` : dir.status === "archived" ? `${dir.files ?? 0} members in ${dir.archive ?? "the render archive"}` : dir.status === "absent" ? "absent" : "unreadable"}
            </span>
          ))}
        </p>
      ) : null}

      {row ? (
        <KitRowEvidence installDir={installDir} row={row} sockGrammar={d.sock_grammar} onReplaceLoose={onReplaceLoose} onReplaceArchived={(a, m, l) => void archiveReplace.pick(a, m, l)} />
      ) : spare ? (
        <div className="space-y-1" data-testid="kits-selected-spare">
          <p className="text-slate-300">
            No jersey row points at kit ({spare.style}, {spare.variant}); it is a shipped spare candidate{" "}
            <Badge tone={VERDICT_TONE[spare.completeness] ?? ""} testId="kits-selected-verdict">
              {spare.completeness}
            </Badge>
          </p>
          <ComponentList installDir={installDir} components={spare.components} sockGrammar={d.sock_grammar} onReplaceLoose={onReplaceLoose} onReplaceArchived={(a, m, l) => void archiveReplace.pick(a, m, l)} />
        </div>
      ) : (
        <p className="text-slate-500" data-testid="kits-selected-none">
          {d.contract_status === "measured"
            ? "The selected kit is neither a registered row nor a shipped candidate under this team's art id."
            : "Automatic kit assignment is not verified for this title. Available texture files are shown separately below."}
        </p>
      )}

      <p className="text-slate-400" data-testid="kits-summary">
        {rows.length} kit row{rows.length === 1 ? "" : "s"}: {count(rows, (k) => k.completeness, "complete")} complete ·{" "}
        {count(rows, (k) => k.completeness, "incomplete")} incomplete · {count(rows, (k) => k.completeness, "unknown")} unknown
        {refused > 0 ? ` · ${refused} refused` : ""}
      </p>

      <div>
        <button
          type="button"
          className="rounded border border-slate-700 px-2 py-0.5 text-slate-300 hover:bg-slate-800"
          onClick={() => setShowSpares((v) => !v)}
          data-testid="kits-spares-toggle"
        >
          {d.spares.length} shipped spare candidate{d.spares.length === 1 ? "" : "s"} no row points at:{" "}
          {count(d.spares, (sp) => sp.completeness, "complete")} complete · {count(d.spares, (sp) => sp.completeness, "incomplete")} incomplete ·{" "}
          {count(d.spares, (sp) => sp.completeness, "unknown")} unknown {showSpares ? "▾" : "▸"}
        </button>
        {showSpares && d.spares.length > 0 ? (
          <ul className="mt-1 space-y-0.5" data-testid="kits-spares">
            {d.spares.map((sp) => (
              <li key={`${sp.style}_${sp.variant}`} className="flex flex-wrap items-center gap-1 text-slate-300" data-testid={`kits-spare-${sp.style}-${sp.variant}`}>
                <span>
                  style {sp.style}, variant {sp.variant}
                </span>
                <Badge tone={VERDICT_TONE[sp.completeness] ?? ""}>{sp.completeness}</Badge>
                {sp.completeness !== "complete" ? (
                  <span className="text-slate-500">
                    {sp.components
                      .filter((c) => c.status !== "present")
                      .map((c) => `${c.label} ${c.status}`)
                      .join(", ")}
                  </span>
                ) : null}
              </li>
            ))}
          </ul>
        ) : null}
      </div>

      <p className="text-slate-500" data-testid="kits-reassignment">
        Re-pointing a row is not offered: {d.reassignment_reason}
      </p>

      {archiveReplace.dialog}
      {archiveReplace.result}

      {d.notes.length > 0 ? (
        <ul className="list-disc space-y-0.5 pl-4 text-slate-500" data-testid="kits-notes">
          {d.notes.map((n, i) => (
            <li key={i}>{n}</li>
          ))}
        </ul>
      ) : null}
    </section>
  );
}

function KitRowEvidence({
  installDir,
  row,
  sockGrammar,
  onReplaceLoose,
  onReplaceArchived,
}: {
  installDir: string | null;
  row: KitDto;
  sockGrammar: string | null;
  onReplaceLoose?: (relPath: string, label: string) => void;
  onReplaceArchived?: (archiveRelPath: string, memberPath: string, label: string) => void;
}) {
  const flags = [
    row.is_default && "default",
    row.is_light_color && "light colour",
    row.has_tie && "tie",
    row.helmet_logo && "helmet logo",
    row.helmet_numbers && "helmet numbers",
  ].filter(Boolean) as string[];
  return (
    <div className="space-y-1" data-testid="kits-selected-row">
      <p className="flex flex-wrap items-center gap-2 text-slate-300">
        <span>
          Row {row.record} · style {row.style}, art id {row.team_art_id}, variant {row.variant}
        </span>
        {row.refusal ? (
          <Badge tone={VERDICT_TONE.incomplete} testId="kits-selected-verdict">
            refused
          </Badge>
        ) : (
          <Badge tone={VERDICT_TONE[row.completeness] ?? ""} testId="kits-selected-verdict">
            {row.completeness}
          </Badge>
        )}
        {flags.length > 0 ? <span className="text-slate-500">{flags.join(" · ")}</span> : null}
      </p>
      {row.refusal ? (
        <p className="text-rose-300" data-testid="kits-selected-refusal">
          {row.refusal}
        </p>
      ) : (
        <ComponentList installDir={installDir} components={row.components} sockGrammar={sockGrammar} onReplaceLoose={onReplaceLoose} onReplaceArchived={onReplaceArchived} />
      )}
      {/* R101 / R103: the nameplate sheet the row's triple names — a naming association on the measured
          titles, shown beside the kit and never counted in its completeness. */}
      {!row.refusal && row.nameplate && (
        <p
          className="flex items-center gap-2 text-slate-400"
          data-testid="kits-nameplate"
          title={row.nameplate.reason ?? row.nameplate.archive_member ?? row.nameplate.path ?? undefined}
        >
          <span className="w-24 shrink-0">nameplate</span>
          <span className="font-mono text-slate-300">{row.nameplate.file_stem}</span>
          <Badge tone={STATUS_TONE[row.nameplate.status] ?? ""} testId="kits-nameplate-status">
            {row.nameplate.status}
          </Badge>
          {row.nameplate.status === "present" && row.nameplate.rel_path && onReplaceLoose && <button type="button" className="rounded border border-slate-700 px-2 py-1 text-sky-300 hover:bg-slate-800" onClick={() => onReplaceLoose(row.nameplate!.rel_path!, "Nameplate")}>Replace…</button>}
        </p>
      )}
    </div>
  );
}

function ComponentList({
  installDir,
  components,
  sockGrammar,
  onReplaceLoose,
  onReplaceArchived,
}: {
  installDir: string | null;
  components: KitComponentDto[];
  sockGrammar: string | null;
  onReplaceLoose?: (relPath: string, label: string) => void;
  onReplaceArchived?: (archiveRelPath: string, memberPath: string, label: string) => void;
}) {
  if (components.length === 0) {
    return <p className="text-slate-500">No components are joined on this title.</p>;
  }
  // The NHL 11 / 12 sock pair: two files, one kit, per-pair mirror class unmeasured (R74 aggregates).
  const pairedSock = (c: KitComponentDto) => c.part === "sock" && sockGrammar != null && sockGrammar !== "single_file";
  const testKey = (c: KitComponentDto) => c.label.replace(/[^a-z0-9]+/g, "-");
  return (
    <ul className="space-y-1">
      {components.map((c) => (
        <li key={c.label} className="flex items-center gap-2" data-testid={`kit-component-${c.label.replace(/[^a-z0-9]+/g, "-")}`} title={c.reason ?? c.archive_member ?? c.path ?? undefined}>
          {installDir && c.status === "present" && c.rel_path ? (
            <TextureThumbnail installDir={installDir} relPath={c.rel_path} className="h-8 w-8 rounded" />
          ) : installDir && c.status === "present" && c.archive_rel_path && c.member_path ? (
            <span data-testid={`kit-component-archived-${c.label.replace(/[^a-z0-9]+/g, "-")}`} title={`decoded from ${c.archive_member ?? "the render archive"}`}>
              <ArchiveMemberThumbnail installDir={installDir} archiveRelPath={c.archive_rel_path} memberPath={c.member_path} className="h-8 w-8 rounded" />
            </span>
          ) : (
            <span className="inline-block h-8 w-8 rounded border border-dashed border-slate-700" aria-hidden />
          )}
          <span className="w-24 shrink-0 text-slate-200">{c.label}</span>
          <Badge tone={STATUS_TONE[c.status] ?? ""} testId={`kit-component-status-${c.label.replace(/[^a-z0-9]+/g, "-")}`}>
            {c.status}
          </Badge>
          <code className="truncate text-slate-400">{c.rel_path ?? c.archive_member ?? c.file_stem}</code>
          {c.reason ? <span className="truncate text-slate-500">— {c.reason}</span> : null}
          {c.status === "present" && pairedSock(c) ? (
            <span className="truncate text-slate-500" data-testid={`kit-component-pair-${testKey(c)}`} title="Both sock files are required parts of one kit; which pairs are exact mirrors is not measured per pair (research R74 gives aggregate classes only), so a single file is not replaced on its own.">
              — pair; not replaced file-by-file
            </span>
          ) : c.status === "present" && c.rel_path && onReplaceLoose ? (
            <button
              type="button"
              onClick={() => onReplaceLoose(c.rel_path!, c.label)}
              className="ml-auto shrink-0 rounded border border-slate-700 px-1.5 py-px text-[10px] text-slate-400 hover:border-sky-700 hover:text-sky-300"
              data-testid={`kit-component-replace-${testKey(c)}`}
            >
              Replace…
            </button>
          ) : c.status === "present" && c.archive_rel_path && c.member_path && onReplaceArchived ? (
            <button
              type="button"
              onClick={() => onReplaceArchived(c.archive_rel_path!, c.member_path!, c.label)}
              title="Replace the archive member: the change is planned now and written into the archive copy by the next copied-target save"
              className="ml-auto shrink-0 rounded border border-slate-700 px-1.5 py-px text-[10px] text-slate-400 hover:border-sky-700 hover:text-sky-300"
              data-testid={`kit-component-replace-archived-${testKey(c)}`}
            >
              Replace in archive…
            </button>
          ) : null}
        </li>
      ))}
    </ul>
  );
}
