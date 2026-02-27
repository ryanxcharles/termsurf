#!/usr/bin/env bash
# rename-ghostty.sh — Rename all "ghostty" references in gui/ to "termsurf"
# Re-runnable after upstream Ghostty merges.
set -euo pipefail

GUI_DIR="${1:-gui}"
export LC_ALL=C
cd "$(git rev-parse --show-toplevel)"

if [ ! -d "$GUI_DIR" ]; then
  echo "Error: $GUI_DIR/ not found"
  exit 1
fi

echo "=== rename-ghostty.sh ==="
echo "Renaming ghostty → termsurf in $GUI_DIR/"
echo ""

# ─────────────────────────────────────────────────────────────────────
# Phase 1+2+3: Protect → Substitute → Restore (single sed pass)
# ─────────────────────────────────────────────────────────────────────
echo "--- Phase 1+2+3: Text substitutions ---"

SED_SCRIPT=$(mktemp)
trap 'rm -f "$SED_SCRIPT"' EXIT

cat > "$SED_SCRIPT" << 'SEDEOF'
# ── Phase 1: Protect (longer/more-specific patterns first) ──
s|ghostty-themes|__PROTECT_01__|g
s|ghostty-org/ghostty|__PROTECT_02__|g
s|mitchellh/ghostty|__PROTECT_03__|g
s|ghostty-org|__PROTECT_04__|g
s|deps\.files\.ghostty\.org|__PROTECT_05__|g
s|release\.files\.ghostty\.org|__PROTECT_06__|g
s|tip\.files\.ghostty\.org|__PROTECT_07__|g
s|ghostty\.cachix\.org|__PROTECT_08__|g
s|discord\.gg/ghostty|__PROTECT_09__|g
s|snapcraft\.io/ghostty|__PROTECT_10__|g
s|Ghostty contributors|__PROTECT_11__|g
s|namespace-profile-ghostty|__PROTECT_12__|g
s|config\.ghostty|__PROTECT_13__|g
s|theme\.ghostty|__PROTECT_14__|g
s|\.ghosttycrash|__PROTECT_15__|g
s|\*\.ghostty|__PROTECT_16__|g
s|appendingPathExtension("ghostty")|__PROTECT_17__|g
s|ghostty\.qcow2|__PROTECT_18__|g
s|\.ghostty\.png|__PROTECT_19__|g

# ── Phase 2: Substitute (specific before generic) ──
s|com\.mitchellh\.ghostty|com.termsurf|g
s|/com/mitchellh/ghostty|/com/termsurf|g
s|ghostty\.org|termsurf.com|g
s|GHOSTTY_|TERMSURF_|g
s|GHOSTTY|TERMSURF|g
s|Ghostty|TermSurf|g
s|ghostty|termsurf|g

# ── Phase 3: Restore ──
s|__PROTECT_01__|ghostty-themes|g
s|__PROTECT_02__|ghostty-org/ghostty|g
s|__PROTECT_03__|mitchellh/ghostty|g
s|__PROTECT_04__|ghostty-org|g
s|__PROTECT_05__|deps.files.ghostty.org|g
s|__PROTECT_06__|release.files.ghostty.org|g
s|__PROTECT_07__|tip.files.ghostty.org|g
s|__PROTECT_08__|ghostty.cachix.org|g
s|__PROTECT_09__|discord.gg/ghostty|g
s|__PROTECT_10__|snapcraft.io/ghostty|g
s|__PROTECT_11__|Ghostty contributors|g
s|__PROTECT_12__|namespace-profile-ghostty|g
s|__PROTECT_13__|config.ghostty|g
s|__PROTECT_14__|theme.ghostty|g
s|__PROTECT_15__|.ghosttycrash|g
s|__PROTECT_16__|*.ghostty|g
s|__PROTECT_17__|appendingPathExtension("ghostty")|g
s|__PROTECT_18__|ghostty.qcow2|g
s|__PROTECT_19__|.ghostty.png|g
SEDEOF

# Binary extensions to skip
BINARY_RE='\.(png|ico|icns|jpg|jpeg|gif|bmp|webp|pdf|a|o|dylib|so|metallib|wasm|ttf|otf|woff|woff2|gz|tar|zip|tgz|xz|zst|pyc|class|jar|dmp|DS_Store)$'

count=0
while IFS= read -r file; do
  if grep -qil ghostty "$file" 2>/dev/null; then
    sed -i '' -f "$SED_SCRIPT" "$file"
    count=$((count + 1))
  fi
done < <(git ls-files "$GUI_DIR" | grep -v -E "$BINARY_RE")

echo "Processed $count files."
echo ""

# ─────────────────────────────────────────────────────────────────────
# Phase 4: File renames (git mv, idempotent)
# ─────────────────────────────────────────────────────────────────────
echo "--- Phase 4: File renames ---"

safe_mv() {
  if [ -e "$1" ]; then
    if git mv "$1" "$2" 2>/dev/null; then
      echo "  $1 → $2"
    else
      echo "  SKIP (untracked): $1"
    fi
  fi
}

# --- include/ ---
safe_mv "$GUI_DIR/include/ghostty.h" "$GUI_DIR/include/termsurf.h"
safe_mv "$GUI_DIR/include/ghostty"   "$GUI_DIR/include/termsurf"

# --- src/ ---
safe_mv "$GUI_DIR/src/main_ghostty.zig"    "$GUI_DIR/src/main_termsurf.zig"
safe_mv "$GUI_DIR/src/cli/ghostty.zig"     "$GUI_DIR/src/cli/termsurf.zig"
safe_mv "$GUI_DIR/src/terminfo/ghostty.zig" "$GUI_DIR/src/terminfo/termsurf.zig"

# --- src/build/ (13 Ghostty*.zig files) ---
for f in \
  GhosttyBench GhosttyDist GhosttyDocs GhosttyExe GhosttyFrameData \
  GhosttyI18n GhosttyLib GhosttyLibVt GhosttyResources GhosttyWebdata \
  GhosttyXCFramework GhosttyXcodebuild GhosttyZig; do
  new=$(echo "$f" | sed 's/Ghostty/TermSurf/')
  safe_mv "$GUI_DIR/src/build/${f}.zig" "$GUI_DIR/src/build/${new}.zig"
done

# --- src/build/mdgen/ ---
for f in \
  ghostty_1_footer.md ghostty_1_header.md \
  ghostty_5_footer.md ghostty_5_header.md \
  main_ghostty_1.zig  main_ghostty_5.zig; do
  new=$(echo "$f" | sed 's/ghostty/termsurf/')
  safe_mv "$GUI_DIR/src/build/mdgen/$f" "$GUI_DIR/src/build/mdgen/$new"
done

# --- shell-integration/ ---
safe_mv "$GUI_DIR/src/shell-integration/bash/ghostty.bash" \
        "$GUI_DIR/src/shell-integration/bash/termsurf.bash"
safe_mv "$GUI_DIR/src/shell-integration/elvish/lib/ghostty-integration.elv" \
        "$GUI_DIR/src/shell-integration/elvish/lib/termsurf-integration.elv"
safe_mv "$GUI_DIR/src/shell-integration/fish/vendor_conf.d/ghostty-shell-integration.fish" \
        "$GUI_DIR/src/shell-integration/fish/vendor_conf.d/termsurf-shell-integration.fish"
safe_mv "$GUI_DIR/src/shell-integration/nushell/vendor/autoload/ghostty.nu" \
        "$GUI_DIR/src/shell-integration/nushell/vendor/autoload/termsurf.nu"
safe_mv "$GUI_DIR/src/shell-integration/zsh/ghostty-integration" \
        "$GUI_DIR/src/shell-integration/zsh/termsurf-integration"

# --- dist/ ---
safe_mv "$GUI_DIR/dist/doxygen/ghostty.css" \
        "$GUI_DIR/dist/doxygen/termsurf.css"
safe_mv "$GUI_DIR/dist/linux/com.mitchellh.ghostty.metainfo.xml.in" \
        "$GUI_DIR/dist/linux/com.termsurf.metainfo.xml.in"
safe_mv "$GUI_DIR/dist/linux/ghostty_dolphin.desktop" \
        "$GUI_DIR/dist/linux/termsurf_dolphin.desktop"
safe_mv "$GUI_DIR/dist/linux/ghostty_nautilus.py" \
        "$GUI_DIR/dist/linux/termsurf_nautilus.py"
safe_mv "$GUI_DIR/dist/windows/ghostty.ico" \
        "$GUI_DIR/dist/windows/termsurf.ico"
safe_mv "$GUI_DIR/dist/windows/ghostty.manifest" \
        "$GUI_DIR/dist/windows/termsurf.manifest"
safe_mv "$GUI_DIR/dist/windows/ghostty.rc" \
        "$GUI_DIR/dist/windows/termsurf.rc"

# --- flatpak/ ---
safe_mv "$GUI_DIR/flatpak/com.mitchellh.ghostty-debug.yml" \
        "$GUI_DIR/flatpak/com.termsurf-debug.yml"
safe_mv "$GUI_DIR/flatpak/com.mitchellh.ghostty.yml" \
        "$GUI_DIR/flatpak/com.termsurf.yml"

# --- images/ (file first, then directory) ---
safe_mv "$GUI_DIR/images/Ghostty.icon/Assets/Ghostty.png" \
        "$GUI_DIR/images/Ghostty.icon/Assets/TermSurf.png"
safe_mv "$GUI_DIR/images/Ghostty.icon" \
        "$GUI_DIR/images/TermSurf.icon"

# --- po/ ---
safe_mv "$GUI_DIR/po/com.mitchellh.ghostty.pot" \
        "$GUI_DIR/po/com.termsurf.pot"

# --- .github/ ---
safe_mv "$GUI_DIR/.github/scripts/ghostty-tip" \
        "$GUI_DIR/.github/scripts/termsurf-tip"

# --- macos/Xcode scheme (file inside project bundle) ---
safe_mv "$GUI_DIR/macos/Ghostty.xcodeproj/xcshareddata/xcschemes/Ghostty.xcscheme" \
        "$GUI_DIR/macos/Ghostty.xcodeproj/xcshareddata/xcschemes/TermSurf.xcscheme"

# --- macos/entitlements ---
safe_mv "$GUI_DIR/macos/GhosttyDebug.entitlements" \
        "$GUI_DIR/macos/TermSurfDebug.entitlements"
safe_mv "$GUI_DIR/macos/GhosttyReleaseLocal.entitlements" \
        "$GUI_DIR/macos/TermSurfReleaseLocal.entitlements"

# --- macos/UI tests (files first, then directory) ---
for f in \
  GhosttyCustomConfigCase.swift GhosttyThemeTests.swift \
  GhosttyTitlebarTabsUITests.swift GhosttyTitleUITests.swift; do
  new=$(echo "$f" | sed 's/Ghostty/TermSurf/')
  safe_mv "$GUI_DIR/macos/GhosttyUITests/$f" "$GUI_DIR/macos/GhosttyUITests/$new"
done
safe_mv "$GUI_DIR/macos/GhosttyUITests" \
        "$GUI_DIR/macos/TermSurfUITests"

# --- macos/Sources/App ---
safe_mv "$GUI_DIR/macos/Sources/App/macOS/AppDelegate+Ghostty.swift" \
        "$GUI_DIR/macos/Sources/App/macOS/AppDelegate+TermSurf.swift"

# --- macos/Sources/Features (files first, then directory) ---
safe_mv "$GUI_DIR/macos/Sources/Features/App Intents/GhosttyIntentError.swift" \
        "$GUI_DIR/macos/Sources/Features/App Intents/TermSurfIntentError.swift"
safe_mv "$GUI_DIR/macos/Sources/Features/Colorized Ghostty Icon/ColorizedGhosttyIcon.swift" \
        "$GUI_DIR/macos/Sources/Features/Colorized Ghostty Icon/ColorizedTermSurfIcon.swift"
safe_mv "$GUI_DIR/macos/Sources/Features/Colorized Ghostty Icon/ColorizedGhosttyIconImage.swift" \
        "$GUI_DIR/macos/Sources/Features/Colorized Ghostty Icon/ColorizedTermSurfIconImage.swift"
safe_mv "$GUI_DIR/macos/Sources/Features/Colorized Ghostty Icon/ColorizedGhosttyIconView.swift" \
        "$GUI_DIR/macos/Sources/Features/Colorized Ghostty Icon/ColorizedTermSurfIconView.swift"
safe_mv "$GUI_DIR/macos/Sources/Features/Colorized Ghostty Icon" \
        "$GUI_DIR/macos/Sources/Features/Colorized TermSurf Icon"

# --- macos/Sources/Ghostty (files first, then directory) ---
for f in \
  Ghostty.Action.swift Ghostty.App.swift Ghostty.Command.swift \
  Ghostty.Config.swift Ghostty.Error.swift Ghostty.Event.swift \
  Ghostty.Input.swift Ghostty.Inspector.swift Ghostty.Shell.swift \
  Ghostty.Surface.swift GhosttyDelegate.swift; do
  new=$(echo "$f" | sed 's/Ghostty/TermSurf/')
  safe_mv "$GUI_DIR/macos/Sources/Ghostty/$f" "$GUI_DIR/macos/Sources/Ghostty/$new"
done
safe_mv "$GUI_DIR/macos/Sources/Ghostty" \
        "$GUI_DIR/macos/Sources/TermSurf"

# --- macos/Tests/Ghostty ---
safe_mv "$GUI_DIR/macos/Tests/Ghostty" \
        "$GUI_DIR/macos/Tests/TermSurf"

# --- macos/Xcode project directory (LAST — parent of scheme) ---
safe_mv "$GUI_DIR/macos/Ghostty.xcodeproj" \
        "$GUI_DIR/macos/TermSurf.xcodeproj"

echo ""

# ─────────────────────────────────────────────────────────────────────
# Phase 5: Verify
# ─────────────────────────────────────────────────────────────────────
echo "--- Phase 5: Verify ---"

echo ""
echo "Checking for leftover __PROTECT_ placeholders..."
leftover=$(git grep -r '__PROTECT_' "$GUI_DIR" 2>/dev/null | head -5 || true)
if [ -n "$leftover" ]; then
  echo "ERROR: Leftover placeholders found:"
  echo "$leftover"
  exit 1
else
  echo "  None found. ✓"
fi

echo ""
echo "Remaining ghostty references (should all be protected patterns):"
remaining=$(git grep -in ghostty "$GUI_DIR" 2>/dev/null || true)
if [ -z "$remaining" ]; then
  echo "  None — fully renamed."
else
  count=$(echo "$remaining" | wc -l | tr -d ' ')
  echo "  $count references remain. Sampling:"
  echo "$remaining" | head -20
  echo ""
  echo "  (Verify these are all protected patterns.)"
fi

echo ""
echo "=== Done ==="
