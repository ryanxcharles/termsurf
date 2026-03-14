import { createFileRoute, Outlet } from "@tanstack/react-router";

export const Route = createFileRoute("/blog")({
  component: BlogLayout,
  head: () => ({
    links: [
      {
        rel: "alternate",
        type: "application/json",
        title: "JSON Feed",
        href: "/blog/feed.json",
      },
      {
        rel: "alternate",
        type: "application/atom+xml",
        title: "Atom Feed",
        href: "/blog/feed.atom.xml",
      },
      {
        rel: "alternate",
        type: "application/rss+xml",
        title: "RSS Feed",
        href: "/blog/feed.rss.xml",
      },
    ],
  }),
});

function BlogLayout() {
  return <Outlet />;
}
