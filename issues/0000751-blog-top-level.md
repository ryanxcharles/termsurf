# Issue 751: Move blog posts to top level

## Goal

Move blog posts from `website/blog-posts/` to a top-level `blog/` directory and
update the website build to use the repo root as the Docker build context. This
proves that the website can include markdown files from anywhere in the repo,
paving the way for top-level `docs/` integration later.

## Background

### Why move blog posts?

The website currently lives in `website/` with blog posts at
`website/blog-posts/`. The Dockerfile builds with `website/` as the context, so
the website can only access files inside that directory.

We want to eventually serve documentation from a top-level `docs/` directory
(like WezTerm's approach). To do that, the Docker build context must be the repo
root. Moving blog posts to the top level first is a smaller, safer change that
proves the pattern works before we tackle docs.

### The earthbucks-com pattern

The earthbucks-com project solves the same problem: a website subdirectory that
needs files from sibling directories. The technique:

1. The deploy script `cd`s to the repo root before running `fly deploy`
2. The Dockerfile uses `COPY` to pull in specific directories from the wider
   context
3. A root-level `.dockerignore` excludes heavy directories (`node_modules`,
   build artifacts)

### What needs to change

1. Move `website/blog-posts/` to `blog/` at the repo root
2. Create a root-level `.dockerignore` that excludes `chromium/`, `vendor/`,
   `wezboard/`, `roamium/`, `webtui/`, and other heavy directories — only allow
   `website/` and `blog/` into the context
3. Update the Dockerfile to build from the repo root context:
   - Copy `website/` for the app
   - Copy `blog/` for blog posts
   - Adjust working directories and paths
4. Update `website/package.json` deploy script:
   `"deploy": "cd .. && fly deploy --config website/fly.toml --dockerfile website/Dockerfile"`
5. Update `website/scripts/build-blog.ts` to read from `../blog/` (or `blog/`
   relative to repo root during Docker build)
6. Update `website/src/server/blog.ts` to read from the correct path at runtime
7. Update `website/vite.config.ts` blog markdown dev middleware to serve from
   the new location
8. Update `website/CLAUDE.md` to reflect the new blog post location
