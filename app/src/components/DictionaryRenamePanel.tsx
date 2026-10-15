import { useEffect, useRef, useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import { Button, Card, Pill } from "./primitives";
import type { DictionaryRepointPreviewDto } from "../bindings/DictionaryRepointPreviewDto";
import type { DictionaryRowDto } from "../bindings/DictionaryRowDto";
import type { DictionaryRetextPreviewDto } from "../bindings/DictionaryRetextPreviewDto";
import type { DictionaryGrowthPreviewDto } from "../bindings/DictionaryGrowthPreviewDto";
import type { DictionaryGrowthOpDto } from "../bindings/DictionaryGrowthOpDto";

/// A preview bound to the inputs and the dictionary it was computed for. It is shown only while
/// those are still the current ones; editing either input or attaching other dictionary bytes
/// makes it obsolete.
type BoundPreview = {
  preview: DictionaryRepointPreviewDto | null;
  retext: DictionaryRetextPreviewDto | null;
  growth: DictionaryGrowthPreviewDto | null;
  key: string;
  donor: string;
  mode: Mode;
  dictionarySha256: string;
};

/// `repoint` = LOC-W1 (show another key's text); `retext` = LOC-W2 (free text re-encoded in place,
/// must fit the old string and use only characters the title's compression tree has);
/// `grow` = LOC-W3 (free text appended to the pool — the file gets longer; for text that does not
/// fit in place); `insert` = LOC-W3 (a new key with its text, at its sorted position; refused on
/// titles whose rows are not in hash order).
type Mode = "repoint" | "retext" | "grow" | "insert";

/// LOC-W1 — rename a league (or any dictionary string) by pointing an existing key at another
/// existing key's text in the title's own localization dictionary. Nothing is typed in: no key is
/// added and no string is encoded; the change is a 4-byte pointer plus the checksum that covers
/// it. Preview is read-only; "Hold rename" makes it the project's pending dictionary change (one
/// at a time), reviewed and saved with the database through the same copied-target transaction.
///
/// Ownership (LEAGUE-PARITY-COMPLETION-R1 F3): every async completion is bound to the request
/// generation and project it started under. The generation is bumped on unmount, on a project
/// change and whenever the inputs or the dictionary identity change, so a late response for
/// inputs that are no longer current — or for a project that is no longer open — is dropped, and
/// the summary is only published for the project this panel still belongs to (the shared
/// `update` boundary rejects a foreign summary as well).
export function DictionaryRenamePanel({
  projectId,
  initialKey,
  keyEvidence,
  dictionarySha256,
}: {
  projectId: number;
  /// The key the selected league is believed to use (PS4: its `stockleagues.name`; gen7: the
  /// vocabulary's traced/hypothesised key), editable.
  initialKey: string;
  keyEvidence: string | null;
  /// Identity of the attached dictionary the preview describes; a change invalidates the preview.
  dictionarySha256: string;
}) {
  const qc = useQueryClient();
  const { update } = useProject();
  const [key, setKey] = useState(initialKey);
  const [donor, setDonor] = useState("TXT_ROW_LEAGUE");
  const [mode, setMode] = useState<Mode>("repoint");
  const [bound, setBound] = useState<BoundPreview | null>(null);
  // LOC-W3 C: the canonical rebuild is never chosen silently. Off by default; the user turns it
  // on (or accepts the offer after a `needs-rebuild` refusal) and the request says so explicitly.
  const [rebuild, setRebuild] = useState(false);

  // ---- ownership guards ------------------------------------------------------------------
  const genRef = useRef(0);
  const mountedRef = useRef(true);
  const projectIdRef = useRef(projectId);
  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      genRef.current += 1;
    };
  }, []);
  // A project change or other dictionary bytes: nothing in flight has an owner any more and the
  // held preview no longer describes what a request would return.
  useEffect(() => {
    if (projectIdRef.current !== projectId) {
      genRef.current += 1;
      projectIdRef.current = projectId;
      setBound(null);
    }
  }, [projectId]);
  useEffect(() => {
    genRef.current += 1;
    setBound((b) => (b && b.dictionarySha256 !== dictionarySha256 ? null : b));
  }, [dictionarySha256]);

  const current = (gen: number, forProject: number) =>
    mountedRef.current && gen === genRef.current && projectIdRef.current === forProject;

  /// The session's dirty flag and revision live in the project summary: refresh it with the
  /// queries — for the project the request was made for, and only if it is still the open one.
  const refresh = async (forProject: number) => {
    void qc.invalidateQueries();
    const summary = await call("project_summary", { projectId: forProject });
    if (mountedRef.current && summary.project_id === projectIdRef.current) update(summary);
  };

  const inputs = () => ({ key: key.trim(), donor: mode === "retext" || mode === "grow" || mode === "insert" ? donor : donor.trim() });
  const setRebuildMode = (on: boolean) => {
    genRef.current += 1;
    setBound(null);
    setRebuild(on);
  };
  const setInput = (which: "key" | "donor", value: string) => {
    // New inputs: the held preview and every in-flight request describe something else now.
    genRef.current += 1;
    setBound(null);
    (which === "key" ? setKey : setDonor)(value);
  };
  const switchMode = (m: Mode) => {
    genRef.current += 1;
    setBound(null);
    setMode(m);
    setDonor(m === "repoint" ? "TXT_ROW_LEAGUE" : "");
  };

  type Fetched = { preview: DictionaryRepointPreviewDto | null; retext: DictionaryRetextPreviewDto | null; growth: DictionaryGrowthPreviewDto | null };
  const growthOps = (key: string, donor: string, m: Mode): DictionaryGrowthOpDto[] =>
    m === "insert" ? [{ kind: "insert", key, text: donor === "" ? null : donor, all_sharers: false }] : [{ kind: "text", key, text: donor, all_sharers: false }];
  const fetchPreview = async (forProject: number, key: string, donor: string, m: Mode, set: boolean): Promise<Fetched> => {
    if (m === "retext") {
      const cmd = set ? "dictionary_retext_set" : "dictionary_retext_preview";
      return { preview: null, retext: await call(cmd, { projectId: forProject, key, text: donor }), growth: null };
    }
    if (m === "grow" || m === "insert") {
      const cmd = set ? "dictionary_growth_set" : "dictionary_growth_preview";
      const options = rebuild ? { serializer: "rebuild" } : null;
      return { preview: null, retext: null, growth: await call(cmd, { projectId: forProject, ops: growthOps(key, donor, m), options }) };
    }
    const cmd = set ? "dictionary_rename_set" : "dictionary_rename_preview";
    return { preview: await call(cmd, { projectId: forProject, key, donor }), retext: null, growth: null };
  };

  const doPreview = useMutation({
    mutationFn: async () => {
      const gen = genRef.current;
      const forProject = projectId;
      const { key, donor } = inputs();
      const m = mode;
      const fetched = await fetchPreview(forProject, key, donor, m, false);
      return { ...fetched, key, donor, mode: m, gen, forProject };
    },
    onSuccess: ({ preview, retext, growth, key, donor, mode: m, gen, forProject }) => {
      if (!current(gen, forProject)) return;
      setBound({ preview, retext, growth, key, donor, mode: m, dictionarySha256 });
    },
  });
  const doSet = useMutation({
    mutationFn: async () => {
      const gen = genRef.current;
      const forProject = projectId;
      const { key, donor } = inputs();
      const m = mode;
      const fetched = await fetchPreview(forProject, key, donor, m, true);
      return { ...fetched, key, donor, mode: m, gen, forProject };
    },
    onSuccess: async ({ preview, retext, growth, key, donor, mode: m, gen, forProject }) => {
      // The edit is held server-side for `forProject` regardless; the panel only adopts the
      // preview when it is still the owner, and the summary only for the project still open.
      if (current(gen, forProject)) setBound({ preview, retext, growth, key, donor, mode: m, dictionarySha256 });
      await refresh(forProject);
    },
  });
  const doClear = useMutation({
    mutationFn: async () => {
      const gen = genRef.current;
      const forProject = projectId;
      await call("dictionary_rename_clear", { projectId: forProject });
      return { gen, forProject };
    },
    onSuccess: async ({ gen, forProject }) => {
      if (current(gen, forProject)) setBound(null);
      await refresh(forProject);
    },
  });
  const busy = doPreview.isPending || doSet.isPending || doClear.isPending;
  const err = doPreview.error || doSet.error || doClear.error;
  // Shown only while it describes the current inputs, mode and dictionary.
  const live =
    bound && bound.key === key.trim() && bound.donor === inputs().donor && bound.mode === mode && bound.dictionarySha256 === dictionarySha256 && (bound.growth === null || (bound.growth.serializer === "canonical-rebuild") === rebuild)
      ? bound
      : null;
  const needsRebuild = !!err && String(err).includes("needs-rebuild") && !rebuild && (mode === "grow" || mode === "insert");
  const preview = live?.preview ?? null;
  const retext = live?.retext ?? null;
  const growth = live?.growth ?? null;
  const canSubmit = !busy && !!key.trim() && (mode === "insert" || !!donor.trim());

  return (
    <Card
      title="Rename via dictionary"
      subtitle={
        mode === "repoint"
          ? "Point an existing key at another existing key's text. No new text is added; the game's own lookup decides what each screen shows."
          : mode === "retext"
            ? "Re-encode this key's own text in place with the title's compression tree. It must fit the old string and use only characters the tree has; nothing else in the file moves."
            : mode === "grow"
              ? "Append new text for this key to the end of the dictionary's string pool and point the key at it. The file gets longer; everything that exists stays where it is. Whether the game reads a grown dictionary is a runtime question."
              : "Insert a new key (with its text, or no text) at its sorted position. Offered only on titles whose rows are in hash order; whether the game finds a new key is a runtime question."
      }
    >
      <p className="mb-3 text-xs text-amber-200">
        Only one dictionary change can be pending. Holding another replaces the previous
        unsaved dictionary change. Save each change before editing another string.
      </p>
      <div className="mb-2 flex items-center gap-2 text-xs">
        <button
          type="button"
          className={"rounded border px-2 py-0.5 " + (mode === "repoint" ? "border-sky-500 text-sky-200" : "border-slate-700 text-slate-300 hover:border-slate-500")}
          onClick={() => switchMode("repoint")}
        >
          text of another key
        </button>
        <button
          type="button"
          className={"rounded border px-2 py-0.5 " + (mode === "retext" ? "border-sky-500 text-sky-200" : "border-slate-700 text-slate-300 hover:border-slate-500")}
          onClick={() => switchMode("retext")}
        >
          free text (fits in place)
        </button>
        <button
          type="button"
          className={"rounded border px-2 py-0.5 " + (mode === "grow" ? "border-sky-500 text-sky-200" : "border-slate-700 text-slate-300 hover:border-slate-500")}
          onClick={() => switchMode("grow")}
          data-testid="mode-grow"
        >
          free text (append, file grows)
        </button>
        <button
          type="button"
          className={"rounded border px-2 py-0.5 " + (mode === "insert" ? "border-sky-500 text-sky-200" : "border-slate-700 text-slate-300 hover:border-slate-500")}
          onClick={() => switchMode("insert")}
          data-testid="mode-insert"
        >
          new key
        </button>
      </div>
      <div className="grid gap-2 sm:grid-cols-[1fr_auto_1fr]">
        <label className="text-xs text-slate-400">
          Key to change{" "}
          {keyEvidence && (
            <Pill tone={keyEvidence === "traced" ? "ok" : "warn"}>{`binding ${keyEvidence}`}</Pill>
          )}
          <input
            aria-label="dictionary key to change"
            value={key}
            onChange={(e) => setInput("key", e.target.value)}
            className="mt-1 w-full rounded-md border border-slate-700 bg-slate-950 px-2 py-1 font-mono text-[12px] text-slate-100 focus:border-sky-600 focus:outline-none"
          />
        </label>
        <span className="self-end pb-1 text-slate-500">{mode === "repoint" ? "← text of" : "← reads"}</span>
        <label className="text-xs text-slate-400">
          {mode === "repoint" ? "Donor key (its text is shown instead)" : mode === "retext" ? "New text (encoded in place)" : mode === "grow" ? "New text (appended)" : "Text of the new key (empty = no text)"}
          <input
            aria-label={mode === "repoint" ? "donor dictionary key" : "new text"}
            value={donor}
            onChange={(e) => setInput("donor", e.target.value)}
            className="mt-1 w-full rounded-md border border-slate-700 bg-slate-950 px-2 py-1 font-mono text-[12px] text-slate-100 focus:border-sky-600 focus:outline-none"
          />
        </label>
      </div>
      <div className="mt-2 flex items-center gap-2">
        <Button variant="secondary" onClick={() => doPreview.mutate()} disabled={!canSubmit}>
          Preview
        </Button>
        <Button onClick={() => doSet.mutate()} disabled={!canSubmit}>
          {mode === "grow" || mode === "insert" ? "Hold growth" : "Hold rename"}
        </Button>
        <Button variant="secondary" onClick={() => doClear.mutate()} disabled={busy}>
          Clear pending rename
        </Button>
      </div>
      {(mode === "grow" || mode === "insert") && (
        <label className="mt-2 flex items-start gap-2 text-xs text-slate-400">
          <input type="checkbox" checked={rebuild} onChange={(e) => setRebuildMode(e.target.checked)} data-testid="growth-rebuild" className="mt-0.5" />
          <span>
            <span className="text-slate-200">Rebuild the dictionary</span> (canonical rebuild): repack the whole string pool from the decoded rows and rebuild the
            compression tree when the new text needs a symbol it lacks. Nothing is reordered; existing shared strings stay shared. Only needed for new symbols; the
            default appends without touching anything that exists.
          </span>
        </label>
      )}
      {err && <p className="mt-2 text-sm text-rose-400">{String(err)}</p>}
      {needsRebuild && (
        <div className="mt-2 rounded-md border border-amber-800 bg-amber-950/40 px-3 py-2 text-xs text-amber-200" data-testid="growth-needs-rebuild">
          The text needs a symbol this dictionary's compression tree does not have. Adding it means rebuilding the dictionary's string pool and tree — an explicit
          choice, previewed with its own accounting; the game may still lack a font glyph for the symbol.{" "}
          <button type="button" className="underline" onClick={() => setRebuildMode(true)} data-testid="growth-offer-rebuild">
            Turn on the rebuild and preview again
          </button>
        </div>
      )}
      {growth && (
        <div className="mt-3 space-y-2 text-sm" data-testid="rename-preview">
          <p className="text-slate-200">
            {growth.edits.map((e) => (
              <span key={e.row.record}>
                <span className="font-mono text-xs text-amber-200/90">{e.row.key}</span>{" "}
                <span className="text-slate-500">({e.lookup}, row {e.row.record})</span>:{" "}
                <span className="text-slate-400">{e.text_before ?? "<null>"}</span> →{" "}
                <span className="text-emerald-300">{e.text_after ?? "<null>"}</span>{" "}
              </span>
            ))}
            {growth.inserts.map((i) => (
              <span key={i.key}>
                new key <span className="font-mono text-xs text-amber-200/90">{i.key}</span>{" "}
                <span className="text-slate-500">(hash {i.hash.toString(16)}h, inserted at row {i.output_record})</span>:{" "}
                <span className="text-emerald-300">{i.text ?? "<null>"}</span>{" "}
              </span>
            ))}
          </p>
          <p className="text-xs text-slate-500">
            {growth.endian_big ? "big-endian" : "little-endian"} dictionary {growth.dictionary_sha256.slice(0, 12)}… · rows {growth.rows_before} → {growth.rows_after} ·{" "}
            {growth.serializer === "canonical-rebuild" ? "canonical rebuild" : `${growth.appended_bytes} bytes appended`} · order class {growth.order_class}
            {growth.descents.length > 0 ? ` (rows out of hash order at ${growth.descents.slice(0, 6).join(", ")}${growth.descents.length > 6 ? "…" : ""})` : ""}
            {growth.studio_touched ? " · carries earlier Studio edits" : ""}
          </p>
          {growth.serializer === "canonical-rebuild" && (
            <div className="rounded-md border border-amber-800 bg-amber-950/30 px-3 py-2 text-xs text-amber-200" data-testid="growth-rebuild-facts">
              <p>
                <Pill tone="warn">rebuild</Pill> the string pool is repacked
                {growth.new_symbols.length > 0 ? ` and the tree rebuilt for ${growth.new_symbols.map((s) => JSON.stringify(s)).join(", ")}` : ""}
              </p>
              {growth.rebuild_accounting.length > 0 && (
                <ul className="mt-1 list-disc pl-5 text-amber-100/80">
                  {growth.rebuild_accounting.map((l, i) => (
                    <li key={i}>{l}</li>
                  ))}
                </ul>
              )}
            </div>
          )}
          {growth.edits.map((e) =>
            e.sharers.length > 0 ? <RowList key={e.row.record} label={e.all_sharers ? "Keys sharing this string (they take the new text too)" : "Keys sharing this string (they keep the old text)"} rows={e.sharers} /> : null,
          )}
          <ul className="list-disc space-y-1 pl-5 text-xs text-amber-200/80">
            {growth.warnings.map((w, i) => (
              <li key={i}>{w}</li>
            ))}
          </ul>
        </div>
      )}
      {retext && (
        <div className="mt-3 space-y-2 text-sm" data-testid="rename-preview">
          <p className="text-slate-200">
            <span className="font-mono text-xs text-amber-200/90">{retext.source.key}</span>{" "}
            <span className="text-slate-500">({retext.source_lookup}, row {retext.source.record})</span>:{" "}
            <span className="text-slate-400">{retext.text_before}</span> →{" "}
            <span className="text-emerald-300">{retext.text_after}</span>
          </p>
          <p className="text-xs text-slate-500">
            {retext.endian_big ? "big-endian" : "little-endian"} dictionary {retext.dictionary_sha256.slice(0, 12)}… · pool offset{" "}
            {retext.pool_offset.toString(16)}h · {retext.new_bits} of {retext.old_extent_bytes * 8 - 16} bits used in place ({retext.old_bits} before) ·
            the string's own bytes plus the body checksum; every other row unchanged.
          </p>
          <RowList label="Rows carrying the same hash (indistinguishable to the game)" rows={retext.hash_collisions} />
          <RowList label="Keys pointing at this same string (they read the new text too)" rows={retext.sharing_text} />
          <ul className="list-disc space-y-1 pl-5 text-xs text-amber-200/80">
            {retext.warnings.map((w, i) => (
              <li key={i}>{w}</li>
            ))}
          </ul>
        </div>
      )}
      {preview && (
        <div className="mt-3 space-y-2 text-sm" data-testid="rename-preview">
          <p className="text-slate-200">
            <span className="font-mono text-xs text-amber-200/90">{preview.source.key}</span>{" "}
            <span className="text-slate-500">({preview.source_lookup}, row {preview.source.record})</span>:{" "}
            <span className="text-slate-400">{preview.text_before}</span> →{" "}
            <span className="text-emerald-300">{preview.text_after}</span>{" "}
            <span className="text-slate-500">
              from <span className="font-mono text-xs">{preview.donor.key}</span> ({preview.donor_lookup}, row {preview.donor.record})
            </span>
          </p>
          <p className="text-xs text-slate-500">
            {preview.endian_big ? "big-endian" : "little-endian"} dictionary {preview.dictionary_sha256.slice(0, 12)}… ·
            pointer slot {preview.slot_offset.toString(16)}h: {preview.old_pointer.toString(16)}h → {preview.new_pointer.toString(16)}h ·
            4 bytes plus the body checksum; every other row unchanged.
          </p>
          <RowList label="Rows carrying the same hash (indistinguishable to the game)" rows={preview.hash_collisions} />
          <RowList label="Keys that keep the old text" rows={preview.sharing_old_text} />
          <RowList label="Keys that already show the new text" rows={preview.sharing_new_text} />
          <ul className="list-disc space-y-1 pl-5 text-xs text-amber-200/80">
            {preview.warnings.map((w, i) => (
              <li key={i}>{w}</li>
            ))}
          </ul>
        </div>
      )}
    </Card>
  );
}

function RowList({ label, rows }: { label: string; rows: DictionaryRowDto[] }) {
  if (rows.length === 0) return null;
  return (
    <p className="text-xs text-slate-400">
      {label}:{" "}
      {rows.map((r) => (
        <span key={r.record} className="mr-2 font-mono text-[11px] text-slate-300" title={r.text ?? "<null>"}>
          {r.key}
        </span>
      ))}
    </p>
  );
}
