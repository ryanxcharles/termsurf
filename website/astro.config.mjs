import { defineConfig } from "astro/config";
import tailwindcss from "@tailwindcss/vite";
import react from "@astrojs/react";

export default defineConfig({
  output: "static",
  trailingSlash: "never",
  integrations: [react()],
  vite: {
    plugins: [tailwindcss()],
  },
});
