import { createFileRoute, Link } from "@tanstack/react-router";
import { Markdown } from "../../components/Markdown";
import { getBlogPost } from "../../server/blog";

export const Route = createFileRoute("/blog/$slug")({
  loader: ({ params: { slug } }) => getBlogPost({ data: slug }),
  head: ({ loaderData }) => ({
    meta: [{ title: `${loaderData?.title ?? "post"} — TermSurf` }],
  }),
  component: BlogPost,
});

function BlogPost() {
  const post = Route.useLoaderData();

  return (
    <article>
      <header className="mb-6">
        <h1 className="text-lg font-bold text-primary mb-1">{post.title}</h1>
        <div className="text-xs text-muted flex gap-3">
          <span>{post.date}</span>
          <span className="text-success">{post.author}</span>
        </div>
        <div className="mt-3 text-muted text-xs">
          ──────────────────────────────────────────────────────────
        </div>
      </header>
      <div className="prose-termsurf">
        <Markdown content={post.content} />
      </div>
      <footer className="mt-8">
        <div className="text-muted text-xs mb-3">
          ──────────────────────────────────────────────────────────
        </div>
        <Link to="/blog" className="text-sm text-accent hover:text-primary">
          [back to blog]
        </Link>
      </footer>
    </article>
  );
}
