import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import { fileURLToPath, URL } from "node:url";

// Tauri 开发期固定端口：tauri.conf.json 的 devUrl 与此一致
export default defineConfig({
  plugins: [vue()],
  clearScreen: false,
  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url)),
    },
  },
  server: {
    port: 51431,
    strictPort: true,
  },
  build: {
    target: "chrome105",
    minify: "esbuild",
    sourcemap: false,
  },
});
