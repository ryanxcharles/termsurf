#!/usr/bin/env bash
# Build the real Ghostty macOS app (Issue 802 / Exp 3) on a machine whose Xcode
# SDK (26.4) is too new for the zig version Ghostty 1.3.2-dev pins (0.15.2).
#
# Strategy (approach 1, "macOS-only"): zig 0.15.2 can't link the Xcode 26.4 SDK,
# but CAN link the CommandLineTools 26.0 SDK. The macOS app only needs the macOS
# slice of GhosttyKit, so we patch out the iOS xcframework slice (which would need
# an iOS SDK zig 0.15.2 can't link), build the macOS lib + Metal shaders under
# CommandLineTools (with Xcode's metal compiler on PATH), then package the
# xcframework and build the Swift app under Xcode. The app itself is unaltered.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
G="$ROOT/vendor/ghostty"
CONFIG="${1:-Debug}"
ZIG="$("$ROOT/scripts/ghostty-app/setup-zig.sh")"          # pinned zig 0.15.2
CLT=/Library/Developer/CommandLineTools                    # SDK zig 0.15.2 can link
METALDIR="$(dirname "$(xcrun -f metal)")"                  # Xcode's Metal toolchain (CLT lacks it)

cd "$G"
# 1. Apply the macOS-only-xcframework patch (gate the iOS slice on .universal).
grep -q "issue 802 / Exp 3" src/build/GhosttyXCFramework.zig \
  || git apply "$ROOT/scripts/ghostty-app/macos-only-xcframework.patch"

# 2. Build the macOS GhosttyKit lib + Metal shaders under CommandLineTools.
#    (Errors at the final create-xcframework step, which needs Xcode — handled in 3.)
DEVELOPER_DIR="$CLT" PATH="$METALDIR:$PATH" "$ZIG" build \
  -Demit-xcframework=true -Dxcframework-target=native -Demit-macos-app=false || true

LIB="$(ls -t .zig-cache/o/*/libghostty-internal-fat.a 2>/dev/null | head -1)"
[ -f "$LIB" ] || { echo "ERROR: macOS lib not built"; exit 1; }
HDR=""; for d in .zig-cache/o/*/; do
  [ -f "$d/ghostty.h" ] && [ -f "$d/module.modulemap" ] && { HDR="$d"; break; }; done
[ -n "$HDR" ] || { echo "ERROR: headers dir not found"; exit 1; }

# 3. Package the macOS-only xcframework under Xcode (xcodebuild needs Xcode).
rm -rf macos/GhosttyKit.xcframework
xcodebuild -create-xcframework -library "$LIB" -headers "$HDR" -output macos/GhosttyKit.xcframework

# 4. Build the Swift app under Xcode (links GhosttyKit; libSystem 26.4 resolves
#    __availability_version_check at app-link time).
nu macos/build.nu --configuration "$CONFIG"
echo "App: $G/macos/build/$CONFIG/Ghostty.app"
