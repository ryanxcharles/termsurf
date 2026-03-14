import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/blog")({
  component: BlogPage,
});

function BlogPage() {
  return (
    <section>
      <h2 className="text-sm font-bold text-foreground mb-4">
        ┌─ Blog ─┐
      </h2>
      <p className="text-muted text-sm">Coming soon.</p>
    </section>
  );
}
