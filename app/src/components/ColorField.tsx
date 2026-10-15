import { useRef, useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import type { FieldViewDto } from "../bindings/FieldViewDto";

/// Phase 2.1 (C8, 20 Sep 2026) — a team colour cell. The domain hands over the fields that make up
/// the colour: **one** packed 24-bit word (`(r << 16) | (g << 8) | b`, PS4 NHL 17–21) or the
/// `_r`, `_g`, `_b` byte fields in that order (NHL 11–15 / Legacy / PS4 15–16). Both shapes show
/// as a swatch plus a `#rrggbb` hex box; a commit writes every component as **one** operation
/// (`set_cells_int`), so a triple is never left half-written and undoes as one step.
///
/// R1 F2 (20 Sep 2026): the draft, its error and the in-flight mutation belong to **one** target —
/// project, table, record and component fields. The editor below is keyed on that identity, so a
/// change of any part of it (another team with the same colour, another project, another field
/// set) unmounts the old editor with its draft rather than carrying the draft over; the mutation
/// captures its own arguments at dispatch and refreshes the project it was dispatched for.
export function ColorField({
  projectId,
  record,
  components,
  label,
}: {
  projectId: number;
  record: number;
  components: FieldViewDto[];
  label: string;
}) {
  return (
    <ColorEditor
      key={targetKey(projectId, record, components)}
      projectId={projectId}
      record={record}
      components={components}
      label={label}
    />
  );
}

/// The identity a colour draft is bound to.
export function targetKey(projectId: number, record: number, components: FieldViewDto[]): string {
  const table = components[0]?.studio_table ?? "";
  return `${projectId}/${table}/${record}/${components.map((c) => c.studio_field).join("+")}`;
}

type Write = { projectId: number; table: string; record: number; writes: Array<{ field: string; value: number }> };

function ColorEditor({
  projectId,
  record,
  components,
  label,
}: {
  projectId: number;
  record: number;
  components: FieldViewDto[];
  label: string;
}) {
  const qc = useQueryClient();
  const { update } = useProject();
  const current = hexOf(components);
  // The draft starts from the stored colour of this (keyed) target; a later change of the stored
  // colour with the same identity (the refetch after our own commit, an undo) re-seeds it.
  const [text, setText] = useState(current ?? "");
  const [seed, setSeed] = useState(current);
  // R2 F1 (20 Sep 2026): two separate guards. `inFlight` is the request actually pending (no
  // second write is dispatched while it is); `awaitingAck` is the hex of a write that succeeded
  // but whose authoritative value has not been refetched yet — a second Enter/blur of the same hex
  // in that window is a duplicate, not a new edit. `awaitingAck` is released by ANY authoritative
  // change of this target's stored colour: the acknowledgement itself (then the `hex === current`
  // rule applies) or a later undo / reload back to another value, after which the same hex is a
  // genuine new edit again.
  //
  // R3 F1 (21 Sep 2026): the suppression is bound to the particular write AND to the authoritative
  // state it was dispatched against. `authGen` counts the authoritative changes of this target's
  // stored colour; a write records the generation at dispatch and, when it settles, may only arm
  // `awaitingAck` if no authoritative change happened meanwhile. Settlement runs after the
  // invalidation + summary refresh, so an acknowledgement (and an undo) can arrive first; a late
  // completion must never reinstate a guard those newer states already invalidated.
  const inFlight = useRef<{ hex: string; gen: number } | null>(null);
  const awaitingAck = useRef<string | null>(null);
  const authGen = useRef(0);
  if (seed !== current) {
    setSeed(current);
    setText(current ?? "");
    awaitingAck.current = null;
    authGen.current += 1;
  }

  const mut = useMutation({
    // Arguments are captured at dispatch, never read from later props.
    mutationFn: (args: Write) => call("set_cells_int", args),
    onSuccess: async (_r, args) => {
      await qc.invalidateQueries();
      // Refresh the project the write was dispatched for; the shared provider adopts the summary
      // only if that project is still the open one.
      update(await call("project_summary", { projectId: args.projectId }));
    },
    onSettled: (_r, error) => {
      // A failed write leaves nothing to acknowledge: the same hex may be retried. A successful one
      // arms the duplicate guard only while the authoritative state is still the one it was
      // dispatched against; if the acknowledgement (or an undo after it) already arrived, the
      // guard was released by that newer state and stays released.
      const done = inFlight.current;
      if (!error && done !== null && done.gen === authGen.current) awaitingAck.current = done.hex;
      inFlight.current = null;
    },
  });

  const editable = components.length > 0 && components.every((c) => c.editable);
  const valid = /^#?[0-9a-fA-F]{6}$/.test(text.trim());
  const commit = () => {
    if (!editable || !valid) return;
    // While a request is pending nothing else is dispatched; a draft typed meanwhile stays in the
    // box and commits on the next Enter/blur once the request has settled.
    if (inFlight.current !== null) return;
    const hex = normalize(text);
    if (hex === current || hex === awaitingAck.current) return;
    inFlight.current = { hex, gen: authGen.current };
    mut.mutate({ projectId, table: components[0].studio_table, record, writes: writesFor(components, hex) });
  };

  return (
    <label className="block min-w-0" data-testid={`color-cell-${label}`}>
      <span className="block text-[10px] uppercase tracking-wide text-slate-500">{label}</span>
      <span className="mt-0.5 flex items-center gap-2">
        <span
          data-testid="color-swatch"
          className="inline-block h-5 w-5 shrink-0 rounded border border-slate-600"
          style={{ background: current ?? "transparent" }}
          title={current ?? "no colour"}
        />
        <input
          data-testid="color-hex"
          className="w-24 rounded border border-slate-700 bg-slate-900 px-1 py-0.5 font-mono text-xs text-slate-100 disabled:opacity-60"
          value={text}
          disabled={!editable || mut.isPending}
          onChange={(e) => setText(e.target.value)}
          onBlur={commit}
          onKeyDown={(e) => {
            if (e.key === "Enter") commit();
            if (e.key === "Escape") setText(current ?? "");
          }}
          placeholder="#rrggbb"
          spellCheck={false}
        />
      </span>
      {!valid && text !== "" && <span className="block text-[10px] text-amber-400">six hex digits</span>}
      {mut.error && <span className="block text-[10px] text-rose-400">{String(mut.error)}</span>}
    </label>
  );
}

function normalize(text: string): string {
  return `#${text.trim().replace(/^#/, "").toLowerCase()}`;
}

/// The colour the components currently hold, as `#rrggbb`; `null` when a component has no value.
export function hexOf(components: FieldViewDto[]): string | null {
  if (components.length === 1) {
    const v = components[0].decoded;
    if (v == null) return null;
    return `#${(v & 0xffffff).toString(16).padStart(6, "0")}`;
  }
  if (components.length === 3) {
    const parts = components.map((c) => c.decoded);
    if (parts.some((p) => p == null)) return null;
    return `#${parts.map((p) => (p! & 0xff).toString(16).padStart(2, "0")).join("")}`;
  }
  return null;
}

/// The `set_cells_int` writes that store `hex` in `components` (packed word or `_r/_g/_b`).
export function writesFor(components: FieldViewDto[], hex: string): Array<{ field: string; value: number }> {
  const n = parseInt(hex.replace(/^#/, ""), 16);
  if (components.length === 1) return [{ field: components[0].studio_field, value: n & 0xffffff }];
  return [
    { field: components[0].studio_field, value: (n >> 16) & 0xff },
    { field: components[1].studio_field, value: (n >> 8) & 0xff },
    { field: components[2].studio_field, value: n & 0xff },
  ];
}
