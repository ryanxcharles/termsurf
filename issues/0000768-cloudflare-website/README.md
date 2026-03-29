+++
status = "open"
opened = "2026-03-29"
+++

# Issue 768: Migrate termsurf.com from Fly.io to Cloudflare Pages

## Goal

Move termsurf.com hosting from Fly.io (Docker + SSR) to Cloudflare Pages
(static), matching the deployment pattern used by ryanxcharles.com and
astrohacker.com.

## Background

### Current setup (Fly.io)

The TermSurf website (`website/`) uses TanStack Start — a React meta-framework
with SSR, server functions, and file-based routing. It deploys as a Docker
container to Fly.io:

- **Framework:** TanStack Start (React 19 + Vite 7)
- **Rendering:** Server-side rendering with `createServerFn` for runtime data
- **Deploy:** `fly deploy` via multi-stage Dockerfile (Bun build, Bun serve)
- **Region:** iad (single region, auto-stop machines)
- **Blog:** Markdown in top-level `blog/`, content loaded at runtime via server
  function reading files from disk

The SSR server function (`src/server/blog.ts`) reads markdown files from
`../../blog/` at request time. This is the only server-side behavior — all other
data (blog metadata, commit history) is pre-generated as JSON at build time.

### Target setup (Cloudflare Pages)

Both ryanxcharles.com and astrohacker.com follow the same pattern:

- **Framework:** Astro 6 with `output: "static"`
- **Styling:** Tailwind CSS v4 via `@tailwindcss/vite`
- **Build output:** `dist/` (static HTML/CSS/JS)
- **Deploy:** `wrangler pages deploy dist`
- **Config:** No `wrangler.toml` — project name cached locally by wrangler
- **DNS:** Managed in Cloudflare dashboard
- **www redirect:** `public/_redirects` file or Cloudflare dashboard rule

Deploy command from package.json:

```
"deploy": "bun run build && wrangler pages deploy dist"
```

### Why migrate

- **Cost:** Cloudflare Pages is free for static sites. Fly.io bills for machine
  time.
- **Performance:** Cloudflare's CDN serves from 300+ edge locations. Fly.io
  serves from one region (iad).
- **Simplicity:** No Docker, no server process, no health checks. Just static
  files on a CDN.
- **Consistency:** Match the deploy pattern of the other sites.

## Analysis

### What needs to change

The website must become fully static. The one piece of server-side behavior is
blog post content loading — the server function reads markdown from disk at
request time. This must move to build time.

**Two approaches:**

#### Approach A: Convert to Astro

Replace TanStack Start with Astro, matching ryanxcharles.com and astrohacker.com
exactly. Astro's content collections handle markdown natively. The existing
React components can run as Astro islands (`client:load`).

**Pros:**

- Identical stack to the other two sites.
- Astro's content collections are purpose-built for markdown blogs.
- Simpler mental model — one framework across all sites.
- Smaller output (Astro ships zero JS by default, only hydrates islands).

**Cons:**

- Requires rewriting routes, layouts, and the root document.
- TanStack Router's file-based routing doesn't map 1:1 to Astro's.
- The Three.js welcome page needs to be ported as a client island.

#### Approach B: Keep TanStack Start, add static adapter

TanStack Start supports static site generation via `@tanstack/static-adapter`.
The server function for blog content can be replaced with build-time data
loading (read all markdown at build, write rendered HTML to JSON).

**Pros:**

- Minimal code changes — keep existing routes, components, and layouts.
- No framework migration.

**Cons:**

- TanStack Start's static mode is less mature than Astro's.
- Diverges from the other sites' stack.
- Still ships the full React runtime to every page.

### Recommendation

This issue does not prescribe an approach. Experiment 1 should determine which
is more practical.

### Deployment changes (either approach)

Regardless of framework choice:

1. **Add `wrangler` as a dev dependency.**
2. **Add deploy script:**
   `"deploy": "bun run build && wrangler pages deploy dist"`
3. **Create `public/_redirects`:**
   ```
   https://www.termsurf.com/* https://termsurf.com/:splat 301
   ```
4. **First deploy:** `wrangler pages deploy dist` will prompt to create the
   project (name: `termsurf`).
5. **DNS:** In Cloudflare dashboard, add CNAME records for `termsurf.com` and
   `www.termsurf.com` pointing to the Pages deployment.
6. **Remove:** `fly.toml`, `Dockerfile`, `.dockerignore`, and `fly deploy`
   script.
