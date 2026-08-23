import { resolve } from "node:path";
import { defineConfig } from "vite";

export default defineConfig({
	root: resolve(import.meta.dirname),
	server: { host: "127.0.0.1", port: 5185, strictPort: true },
	build: {
		outDir: resolve(import.meta.dirname, "../../../../.build/desktop/editor-browser"),
		emptyOutDir: true,
		rollupOptions: {
			input: {
				textModel: resolve(import.meta.dirname, "textModel.html"),
				documentModel: resolve(import.meta.dirname, "documentModel.html"),
			},
		},
	},
});
