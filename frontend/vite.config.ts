import { sveltekit } from "@sveltejs/kit/vite";
import { defineConfig } from "vite";

export default defineConfig({
	plugins: [sveltekit()],
	server: {
		port: 3000,
		proxy: {
			"/api": "http://localhost:3444",
			"/health": "http://localhost:3444",
		},
	},
});
