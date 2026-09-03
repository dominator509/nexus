import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

/**
 * EP-033 PWA build (AUD-038). The production build compiles the REAL
 * React entry (index.html -> src/main.tsx -> @nexus/ui components)
 * into apps/web/dist with the manifest; the a11y and e2e suites keep
 * proving the same components over the same contracts.
 */
export default defineConfig({
  plugins: [react()],
  build: {
    outDir: "dist",
    emptyOutDir: true,
  },
  server: {
    host: "127.0.0.1",
    port: 4173,
  },
});
