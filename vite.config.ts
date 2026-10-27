import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// https://v2.tauri.app/start/frontend/vite/
export default defineConfig({
  plugins: [svelte()],
  // No ocultar los errores de compilación de Rust que imprime `tauri dev`.
  clearScreen: false,
  server: {
    // Tauri apunta a un puerto fijo (devUrl): si está ocupado, mejor fallar que servir en otro.
    port: 1420,
    strictPort: true,
    host: "127.0.0.1",
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
  build: {
    // WebView2 es Chromium evergreen: no hace falta transpilar para navegadores viejos.
    target: "chrome120",
    outDir: "dist",
  },
});
