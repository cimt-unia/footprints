import { defineConfig } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";
import tailwindcss from '@tailwindcss/vite'

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// vite-plugin-svelte serves a component's `<style>` block from the compile
// cache. If the style module is requested before the component itself has
// been compiled (e.g. a webview reconnecting right after the dev server
// starts), it falls back to the raw .svelte file and @tailwindcss/vite fails
// with "Invalid declaration". Compile the owning component first.
/** @returns {import('vite').Plugin} */
function svelteStyleWarmup() {
	return {
		name: "svelte-style-warmup",
		apply: "serve",
		enforce: "pre",
		async load(id) {
			const [file, query] = id.split("?", 2);
			if (!query || !file.endsWith(".svelte")) return null;
			const params = new URLSearchParams(query);
			if (!params.has("svelte") || params.get("type") !== "style") return null;
			// `apply: "serve"` means this only ever runs in a dev environment.
			const env = /** @type {import('vite').DevEnvironment} */ (this.environment);
			await env.transformRequest(file);
			return null;
		},
	};
}

// https://vite.dev/config/
export default defineConfig(async () => ({
	plugins: [svelteStyleWarmup(), sveltekit(), tailwindcss()],

	// Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
	//
	// 1. prevent Vite from obscuring rust errors
	clearScreen: false,
	// 2. tauri expects a fixed port, fail if that port is not available
	server: {
		port: 1420,
		strictPort: true,
		host: host || false,
		hmr: host
			? {
				protocol: "ws",
				host,
				port: 1421,
			}
			: undefined,
		watch: {
			// 3. tell Vite to ignore watching `src-tauri`
			ignored: ["**/src-tauri/**"],
		},
	},
}));
