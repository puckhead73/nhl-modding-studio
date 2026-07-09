import type { FieldViewDto } from "../bindings/FieldViewDto";

// Friendly-edit helpers: let the user type the front-facing value (2011, 209 lb, 6'1") while the
// backend stores the raw encoding. The offset (and the height flag) come from the backend
// (`edit_offset`/`edit_height`), so the raw↔friendly transform stays single-sourced in studio-domain.

/** Format total inches as `6'1"`. */
export function formatFtIn(inches: number): string {
  return `${Math.floor(inches / 12)}'${inches % 12}"`;
}

/** Parse a height entry into total inches. Accepts `6'1"`, `6'1`, `6-1`, `6 1`, or plain inches (`73`). */
export function parseFtIn(input: string): number | null {
  const s = input.trim().replace(/["″]/g, "");
  if (s === "") return null;
  const m = s.match(/^(\d+)\s*['’\-\s]\s*(\d+)$/);
  if (m) return Number(m[1]) * 12 + Number(m[2]);
  return /^\d+$/.test(s) ? Number(s) : null;
}

/** Decimal places implied by a fixed-point scale (0.001 → 3, 0.01 → 2). */
function scaleDecimals(scale: number): number {
  return Math.max(0, Math.round(-Math.log10(scale)));
}

/** The friendly value to seed an editable input with. */
export function friendlySeed(field: FieldViewDto): string {
  if (field.is_string) return field.text ?? "";
  const raw = field.decoded ?? 0;
  if (field.edit_scale != null) {
    return (raw * field.edit_scale).toFixed(scaleDecimals(field.edit_scale));
  }
  if (field.edit_offset != null) {
    const v = raw + field.edit_offset;
    return field.edit_height ? formatFtIn(v) : String(v);
  }
  return String(raw);
}

/** True when a field is a plain bounded integer (no offset/height/enum) — its raw range applies
 *  directly, so the editor can bound input to `[range_low, range_high]` (e.g. the 0–15 biases). */
export function hasPlainBounds(field: FieldViewDto): boolean {
  return (
    !field.is_string &&
    !field.edit_height &&
    field.edit_offset == null &&
    field.edit_scale == null &&
    field.range_low != null &&
    field.range_high != null
  );
}

/** Clamp a raw value into the field's declared `[range_low, range_high]` (no-op when unbounded). */
export function clampRaw(field: FieldViewDto, raw: number): number {
  let v = raw;
  if (field.range_low != null && v < field.range_low) v = field.range_low;
  if (field.range_high != null && v > field.range_high) v = field.range_high;
  return v;
}

/** Convert a friendly input string into the raw value to stage, or `null` if unparseable. */
export function friendlyToRaw(field: FieldViewDto, input: string): number | null {
  if (field.edit_height) {
    const inches = parseFtIn(input);
    return inches == null ? null : inches - (field.edit_offset ?? 0);
  }
  const n = Number(input.trim());
  if (input.trim() === "" || !Number.isFinite(n)) return null;
  if (field.edit_scale != null) return Math.round(n / field.edit_scale);
  return field.edit_offset != null ? n - field.edit_offset : n;
}
