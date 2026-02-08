import {
  defineConfig,
  type PluginOption,
  type UserConfig,
  type ServerOptions,
} from "vite";
import react from "@vitejs/plugin-react";
import path from "path";
import tailwindcss from "@tailwindcss/vite";

const host = process.env.TAURI_DEV_HOST;

// https://vitejs.dev/config/
export default defineConfig(
  async (): Promise<UserConfig> => ({
    plugins: [react(), tailwindcss() as unknown as PluginOption],

    resolve: {
      alias: {
        "@": path.resolve(__dirname, "./src"),
      },
    },

    // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
    //
    // 1. prevent vite from obscuring rust errors
    clearScreen: false,
    // 2. tauri expects a fixed port, fail if that port is not available
    server: {
      port: 1420,
      strictPort: true,
      host: host || false,
      ...(host
        ? {
            hmr: {
              protocol: "ws",
              host,
              port: 1421,
            },
          }
        : {}),
      watch: {
        // 3. tell vite to ignore watching `src-tauri`
        ignored: ["**/src-tauri/**"],
      },
    } as ServerOptions,
  }),
);
