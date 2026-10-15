import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

// Tauri expects a fixed dev port and no auto-clearing of the console.
export default defineConfig({
  plugins: [react(), tailwindcss()],
  clearScreen: false,
  // The About page embeds the vendored extractor licence from the workspace.
  server: { port: 1420, strictPort: true, fs: { allow: [".."] } },
  build: { target: "es2021", outDir: "dist" },
});
