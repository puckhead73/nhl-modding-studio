/** Enum-backed integers are edited by label; their stored IDs stay behind the control. */
export function EnumSelect({ value, options, label, disabled, onChange }: {
  value: number;
  options: { raw: number; label: string }[];
  label: string;
  disabled?: boolean;
  onChange: (raw: number) => void;
}) {
  const known = options.some((option) => option.raw === value);
  return (
    <select
      aria-label={label}
      className="w-full min-w-0 rounded border border-slate-700 bg-slate-950 px-1.5 py-0.5 text-xs text-slate-100 focus:border-sky-600 focus:outline-none"
      value={value}
      disabled={disabled}
      onChange={(event) => onChange(Number(event.target.value))}
    >
      {/* Preserve an uncatalogued existing value until the user deliberately selects a label. */}
      {!known && <option value={value}>Unknown value ({value})</option>}
      {options.map((option) => <option key={option.raw} value={option.raw}>{option.label}</option>)}
    </select>
  );
}
