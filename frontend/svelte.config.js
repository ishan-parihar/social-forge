import adapter from "@sveltejs/adapter-static";
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

/** @type {import('@sveltejs/kit').Config} */
const config = {
	preprocess: vitePreprocess(),
	kit: {
		adapter: adapter({ fallback: "index.html" }),
	},
	// Suppress Svelte warnings that are informational only and should not
	// fail the production build. Fix individually over time.
	onwarn: (warning, handler) => {
		// Accessibility: labels not associated with controls
		if (warning.code.startsWith("a11y_")) return;
		// Svelte 5: $state() initialized from prop reference (reactive pattern)
		if (warning.code === "state_referenced_locally") return;
		handler(warning);
	},
};

export default config;
