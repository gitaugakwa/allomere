import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { cjsInterop } from "vite-plugin-cjs-interop";
import path from "path";

// https://vitejs.dev/config/
export default defineConfig(async () => ({
	plugins: [
		react(),
		cjsInterop({
			dependencies: ["path-browserify"],
		}),
	],

	// Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
	//
	// 1. prevent vite from obscuring rust errors
	clearScreen: false,
	// 2. tauri expects a fixed port, fail if that port is not available
	server: {
		port: 1420,
		strictPort: true,
	},
	resolve: {
		alias: {
			"@": path.resolve(__dirname, "./src"),
			"@app": path.resolve(__dirname, "src/"),
			"@assets": path.resolve(__dirname, "src/assets/"),
			"@constants": path.resolve(__dirname, "src/constants/"),
			"@states": path.resolve(__dirname, "src/states/"),
			"@types": path.resolve(__dirname, "src/types/"),
		},
	},
}));
