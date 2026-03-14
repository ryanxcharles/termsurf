import { useState } from "react";

interface Commit {
  hash: string;
  message: string;
  body: string;
  author: string;
  date: string;
}

interface CommitLogProps {
  commits: Commit[];
}

const GITHUB_REPO = "https://github.com/termsurf/termsurf";

function formatRelativeDate(dateStr: string): string {
  const date = new Date(dateStr);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24));

  if (diffDays === 0) return "today";
  if (diffDays === 1) return "yesterday";
  if (diffDays < 7) return `${diffDays}d ago`;
  if (diffDays < 30) return `${Math.floor(diffDays / 7)}w ago`;
  if (diffDays < 365) return `${Math.floor(diffDays / 30)}mo ago`;
  return `${Math.floor(diffDays / 365)}y ago`;
}

function CommitRow({ commit }: { commit: Commit }) {
  const [expanded, setExpanded] = useState(false);

  return (
    <li>
      <button
        onClick={() => setExpanded(!expanded)}
        className="w-full text-left py-1 flex gap-3 items-baseline hover:bg-background-highlight/30 cursor-pointer text-sm"
      >
        <span className="text-muted">{expanded ? "[-]" : "[+]"}</span>
        <code className="text-secondary">{commit.hash.slice(0, 7)}</code>
        <span className="text-foreground truncate flex-1">{commit.message}</span>
        <span className="text-success">{commit.author}</span>
        <span className="text-muted">{formatRelativeDate(commit.date)}</span>
      </button>

      {expanded && (
        <div className="ml-10 mb-2">
          <pre className="text-xs leading-none whitespace-pre-wrap">
            <span className="text-muted">
              ┌──────────────────────────────────────────────────────{"\n"}
            </span>
            {commit.body && commit.body.length > 0 ? (
              commit.body.split("\n").map((line, i) => (
                <span key={i}>
                  <span className="text-muted">│</span>
                  <span className="text-foreground-dark"> {line}</span>
                  {"\n"}
                </span>
              ))
            ) : (
              <span>
                <span className="text-muted">│</span>
                <span className="text-muted italic"> No commit message</span>
                {"\n"}
              </span>
            )}
            <span>
              <span className="text-muted">│</span>{" "}
              <a
                href={`${GITHUB_REPO}/commit/${commit.hash}`}
                target="_blank"
                rel="noopener noreferrer"
                className="text-accent hover:text-primary"
              >
                [view on GitHub]
              </a>
              {"\n"}
            </span>
            <span className="text-muted">
              └──────────────────────────────────────────────────────
            </span>
          </pre>
        </div>
      )}
    </li>
  );
}

export function CommitLog({ commits }: CommitLogProps) {
  return (
    <section>
      <h2 className="text-sm font-bold text-foreground mb-2">┌─ Recent Commits ─┐</h2>
      <ul>
        {commits.map((commit) => (
          <CommitRow key={commit.hash} commit={commit} />
        ))}
      </ul>
    </section>
  );
}
