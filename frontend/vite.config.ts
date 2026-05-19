import { sveltekit } from "@sveltejs/kit/vite";
import { defineConfig } from "vite";

export default defineConfig({
	plugins: [sveltekit()],
	server: {
		port: 5173,
		proxy: {
			"/api": { target: "https://localhost:6543", secure: false },
			"/health": { target: "https://localhost:6543", secure: false },
			"/setup": { target: "https://localhost:6543", secure: false },
		},
	},
});
