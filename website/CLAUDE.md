# TermSurf Website

The TermSurf website at termsurf.com. Built with Vite, React, TanStack Router,
and Tailwind CSS v4.

## Build Commands

| Command | Purpose |
| --- | --- |
| `bun run dev` | Start dev server |
| `bun run build` | Vite production build |
| `bun run build:blog` | Generate blog.json + feeds from docs/blog/ |
| `bun run build:commits` | Generate commits.json from git history |
| `bun run build:data` | Run all data generators (commits + blog) |

Always run `build:data` before `build` to ensure data files are up to date.

## TanStack Router

### File-based routing

Routes live in `src/routes/`. The Vite plugin generates `src/routeTree.gen.ts`
automatically. Never edit the generated file.

### Naming conventions

| Pattern | Example | URL |
| --- | --- | --- |
| Index | `routes/index.tsx` | `/` |
| Static | `routes/about.tsx` | `/about` |
| Dynamic param | `routes/blog/$slug.tsx` | `/blog/:slug` |
| Layout (with URL) | `routes/blog.tsx` | Wraps all `/blog/*` children |
| Pathless layout | `routes/_app/dashboard.tsx` | `/dashboard` (layout without URL segment) |

### Data loading

This is a static Vite site (no SSR). Data comes from JSON files generated at
build time (`data/blog.json`, `data/commits.json`). Import them directly:

```tsx
import blogData from "../../data/blog.json";
```

Do not use TanStack Router loaders for static data — they are for SSR/dynamic
fetching.

### Route params

Dynamic segments use `$` in filenames. Access with `Route.useParams()`:

```tsx
// routes/blog/$slug.tsx
const { slug } = Route.useParams();
```

Links are type-safe:

```tsx
<Link to="/blog/$slug" params={{ slug: post.slug }}>Read</Link>
```

## Blog

### Writing posts

Blog posts are markdown files in `docs/blog/` (top-level, not in `website/`).

**Filename format:** `YYYY-MM-DD-slug.md`

**Front matter:** TOML between `+++` delimiters.

```toml
+++
title = "Post Title"
author = "Author Name"
date = "YYYY-MM-DD"
+++
```

### Build pipeline

`website/scripts/build-blog.ts` reads all `.md` files from `docs/blog/`,
parses TOML front matter, and produces:

- `website/data/blog.json` — typed metadata + raw markdown content
- `website/public/blog/feed.json` — JSON Feed
- `website/public/blog/feed.atom.xml` — Atom feed
- `website/public/blog/feed.rss.xml` — RSS feed

Posts are sorted newest-first. Run `bun run build:blog` after adding or editing
posts.

### Types

`src/blog.ts` defines `BlogPost` and `BlogData` interfaces. Import these when
working with blog data.

### Markdown rendering

Uses `react-markdown` with the unified plugin chain:

- `remark-smartypants` — smart typography (curly quotes, em-dashes)
- `remark-gfm` — GitHub-flavored markdown (tables, strikethrough)
- `remark-math` — math syntax (`$...$` inline, `$$...$$` display)
- `rehype-katex` — render math to HTML
- `rehype-highlight` — syntax highlighting via highlight.js/lowlight

## Styling

### Theme

Tokyo Night Day (light) is the default. Tokyo Night (dark) activates via
`prefers-color-scheme: dark`. Both are defined in `src/globals.css` using CSS
custom properties that the `@theme` block and a `@media` override control.

**Do not hardcode colors.** Use the semantic variables: `text-primary`,
`bg-background`, `text-muted`, `border-border`, etc. These automatically
switch between light and dark mode.

### Aesthetic

Cypherpunk / brutalist terminal aesthetic:

- **Monospace everything.** Body text, headings, nav — all use `--font-mono`.
- **ASCII borders.** Use box-drawing characters (`────`, `┌─`, `└─`) instead
  of CSS borders for section dividers.
- **No rounded corners.** No `rounded` classes, no border-radius.
- **No shadows.** No drop shadows or box shadows.
- **No smooth transitions** on interactive elements.
- **Bracket links.** Navigation uses `[label]` style. Active page: `>[label]`.
- **Dense layout.** Tight spacing, less padding.
- **Scanline overlay.** Subtle CRT scanlines via `body::after` in globals.css.

### Welcome page

The `/welcome` route is a standalone Three.js 3D experience. It has its own
styling and is excluded from the shared layout in `__root.tsx`. **Do not modify
the welcome page** when changing site-wide styles.
