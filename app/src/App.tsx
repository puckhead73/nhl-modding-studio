import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { DbSummary } from "./bindings/DbSummary";

export function App() {
  const [path, setPath] = useState("");
  const [summary, setSummary] = useState<DbSummary | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function open() {
    setError(null);
    setSummary(null);
    try {
      const result = await invoke<DbSummary>("open_database", { path });
      setSummary(result);
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <main style={{ fontFamily: "system-ui", padding: 24, maxWidth: 900 }}>
      <h1>NHL Modding Studio</h1>
      <p>Open a TDB database (e.g. an anchor <code>nhlng.db</code> / <code>nhl_game.db</code>).</p>
      <div style={{ display: "flex", gap: 8 }}>
        <input
          style={{ flex: 1, padding: 6 }}
          placeholder="path to a .db file"
          value={path}
          onChange={(e) => setPath(e.target.value)}
        />
        <button onClick={open}>Open</button>
      </div>

      {error && <p style={{ color: "crimson" }}>{error}</p>}

      {summary && (
        <section style={{ marginTop: 16 }}>
          <p>
            <b>{summary.endian}</b> · {summary.table_count} tables · {summary.db_size} bytes ·
            header CRC {summary.header_crc_ok ? "OK" : "MISMATCH"} ·
            size {summary.size_ok ? "OK" : "MISMATCH"}
          </p>
          <table style={{ borderCollapse: "collapse", width: "100%" }}>
            <thead>
              <tr>
                <th style={cell}>Table</th>
                <th style={cell}>Fields</th>
                <th style={cell}>Records</th>
              </tr>
            </thead>
            <tbody>
              {summary.tables.slice(0, 50).map((t) => (
                <tr key={t.id}>
                  <td style={cell}>{t.id}</td>
                  <td style={cell}>{t.field_count}</td>
                  <td style={cell}>{t.records}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </section>
      )}
    </main>
  );
}

const cell: React.CSSProperties = {
  border: "1px solid #ccc",
  padding: "4px 8px",
  textAlign: "left",
};
