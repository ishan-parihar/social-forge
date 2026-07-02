import adapter from "@sveltejs/adapter-static";
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

/** @type {import('@sveltejs/kit').Config} */
const config = {
	preprocess: vitePreprocess(),
	kit: {
		adapter: adapter({ fallback: "index.html" }),
	},
	// Suppress a11y warnings — they are informational only and should not
	// fail the production build. Fix individually over time.
	onwarn: (warning, handler) => {
		if (warning.code.startsWith("a11y_")) return;
		handler(warning);
	},
};

export default config;
