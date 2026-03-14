import { createFileRoute } from "@tanstack/react-router";
import { CommitLog } from "../components/CommitLog";
import commitsData from "../../data/commits.json";

export const Route = createFileRoute("/")({
  component: HomePage,
});

function HomePage() {
  return (
    <>
      <section className="mb-8">
        <h2 className="text-sm font-bold text-foreground mb-4">
          ┌─ Latest Post ─┐
        </h2>
        <p className="text-muted text-sm">No posts yet.</p>
      </section>
      <CommitLog commits={commitsData.commits.slice(0, 10)} />
    </>
  );
}
