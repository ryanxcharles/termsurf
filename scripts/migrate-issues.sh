#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(dirname "$SCRIPT_DIR")"
ISSUES_DIR="$REPO_DIR/issues"

cd "$REPO_DIR"

count=0
for file in "$ISSUES_DIR"/*.md; do
  [ -f "$file" ] || continue

  basename="$(basename "$file" .md)"
  folder="$ISSUES_DIR/$basename"

  # Create folder
  mkdir -p "$folder"

  # Determine status: has "## Conclusion" at top level = closed
  if grep -q '^## Conclusion' "$file"; then
    status="closed"
  else
    status="open"
  fi

  # Get opened date (when file was first added to git)
  opened=$(git log --follow --diff-filter=A --format=%aI -- "$file" | tail -1 | cut -dT -f1)
  if [ -z "$opened" ]; then
    # File not yet committed — use today
    opened=$(date +%Y-%m-%d)
  fi

  # Get closed date (last modification) if closed
  closed=""
  if [ "$status" = "closed" ]; then
    closed=$(git log -1 --format=%aI -- "$file" | cut -dT -f1)
    if [ -z "$closed" ]; then
      closed=$(date +%Y-%m-%d)
    fi
  fi

  # Build frontmatter
  if [ "$status" = "closed" ]; then
    frontmatter="+++
status = \"closed\"
opened = \"$opened\"
closed = \"$closed\"
+++"
  else
    frontmatter="+++
status = \"open\"
opened = \"$opened\"
+++"
  fi

  # Prepend frontmatter to file content
  {
    echo "$frontmatter"
    echo ""
    cat "$file"
  } > "$folder/README.md"

  # Remove original file and stage
  git rm --quiet "$file"
  git add "$folder/README.md"

  count=$((count + 1))
  echo "  $basename → $basename/README.md [$status]"
done

echo ""
echo "Migrated $count issues."
