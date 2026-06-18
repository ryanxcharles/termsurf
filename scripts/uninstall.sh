#!/usr/bin/env bash
set -euo pipefail

COMPONENT="${1:-}"
APPLICATIONS_DIR="${TERMSURF_APPLICATIONS_DIR:-/Applications}"

if [ -z "$COMPONENT" ]; then
  echo "Usage: $0 <component>"
  echo "Components: wezboard, ghostboard, roamium, webtui, all"
  exit 1
fi

needs_root() {
  if [ "$COMPONENT" = "ghostboard" ] && [ "$APPLICATIONS_DIR" != "/Applications" ]; then
    mkdir -p "$APPLICATIONS_DIR" || {
      echo "Error: TERMSURF_APPLICATIONS_DIR is not writable: $APPLICATIONS_DIR"
      exit 1
    }
    [ -w "$APPLICATIONS_DIR" ] && return 1
    echo "Error: TERMSURF_APPLICATIONS_DIR is not writable: $APPLICATIONS_DIR"
    exit 1
  fi
  return 0
}

# Re-exec as root so we only prompt for the password once.
if [ "$(id -u)" -ne 0 ] && needs_root; then
  exec sudo "$0" "$@"
fi

LSREGISTER="/System/Library/Frameworks/CoreServices.framework/Versions/A/Frameworks/LaunchServices.framework/Versions/A/Support/lsregister"

uninstall_roamium() {
  echo "==> Uninstalling Roamium..."
  rm -rf /usr/local/roamium
  rm -f /usr/local/bin/roamium
  rm -rf /usr/local/lib/roamium

  echo "  Removed: /usr/local/roamium"
}

uninstall_wezboard() {
  local APP="/Applications/TermSurf Wezboard.app"

  echo "==> Uninstalling Wezboard..."
  rm -rf "$APP"

  echo "  Removed: $APP"
}

uninstall_ghostboard() {
  local APP_DIR="/Applications"
  if [ "$COMPONENT" = "ghostboard" ]; then
    APP_DIR="$APPLICATIONS_DIR"
  fi
  local APP="$APP_DIR/TermSurf Ghostboard.app"

  echo "==> Uninstalling Ghostboard..."
  rm -rf "$APP"

  echo "  Removed: $APP"
}

uninstall_webtui() {
  echo "==> Uninstalling webtui..."
  rm -f /usr/local/bin/web

  echo "  Removed: /usr/local/bin/web"
}

case "$COMPONENT" in
  roamium)    uninstall_roamium ;;
  wezboard)   uninstall_wezboard ;;
  ghostboard) uninstall_ghostboard ;;
  webtui)     uninstall_webtui ;;
  all)
    uninstall_roamium
    uninstall_wezboard
    uninstall_ghostboard
    uninstall_webtui
    echo ""
    echo "Done (all)."
    ;;
  *)
    echo "Unknown component: $COMPONENT"
    echo "Components: wezboard, ghostboard, roamium, webtui, all"
    exit 1
    ;;
esac
