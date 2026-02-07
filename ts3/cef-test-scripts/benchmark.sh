#!/bin/bash
# Benchmark cef-test: build, run for 70s, collect and print performance stats.
#
# Usage:
#   cd ts3 && ./cef-test-scripts/benchmark.sh [--release]
#
# Requires: cef-osr.app already built (build.sh handles this).

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TS3_DIR="$(dirname "$SCRIPT_DIR")"
BUILD_SCRIPT="$SCRIPT_DIR/build.sh"
APP="$TS3_DIR/CefTest.app"
LOG="/tmp/cef-test-gui.log"
DURATION=70

# Parse flags
BUILD_FLAGS=""
MODE="debug"
for arg in "$@"; do
    case $arg in
        --release) BUILD_FLAGS="--release"; MODE="release" ;;
    esac
done

# 1. Kill existing processes
echo "Killing existing cef-test processes..."
pkill -f cef-test-gui 2>/dev/null || true
pkill -f cef-test-profile 2>/dev/null || true
sleep 1

# 2. Build
echo "Building..."
"$BUILD_SCRIPT" $BUILD_FLAGS 2>&1 | tail -1

# 3. Clear log
: > "$LOG"

# 4. Launch
echo "Launching CefTest..."
open --stdout "$LOG" --stderr "$LOG" "$APP"

# 5. Wait for app to start receiving frames
echo -n "Waiting for frames"
for i in $(seq 1 30); do
    if grep -q "PERF-" "$LOG" 2>/dev/null; then
        echo " (started after ${i}s)"
        break
    fi
    echo -n "."
    sleep 1
done

# 6. Run for the benchmark duration
echo "Running benchmark for ${DURATION}s..."
sleep "$DURATION"

# 7. Kill
echo "Stopping..."
pkill -f cef-test-gui 2>/dev/null || true
pkill -f cef-test-profile 2>/dev/null || true
sleep 1

# 8. Extract and print results
echo ""
echo "=== cef-test Benchmark (${DURATION}s, ${MODE}) ==="
echo ""

# Get the last PERF summary lines (the longest-duration, most stable data)
LEFT_STATS=$(grep '\[PERF-LEFT\] frames=' "$LOG" | tail -1)
LEFT_INTERVALS=$(grep '\[PERF-LEFT\] intervals:' "$LOG" | tail -1)
RIGHT_STATS=$(grep '\[PERF-RIGHT\] frames=' "$LOG" | tail -1)
RIGHT_INTERVALS=$(grep '\[PERF-RIGHT\] intervals:' "$LOG" | tail -1)

if [ -z "$LEFT_STATS" ] || [ -z "$RIGHT_STATS" ]; then
    echo "ERROR: No performance data found in $LOG"
    echo "Check the log file for errors."
    exit 1
fi

# Parse LEFT
L_FRAMES=$(echo "$LEFT_STATS" | sed 's/.*frames=\([0-9]*\).*/\1/')
L_DURATION=$(echo "$LEFT_STATS" | sed 's/.*duration=\([0-9.]*\)s.*/\1/')
L_FPS=$(echo "$LEFT_STATS" | sed 's/.*avg_fps=\([0-9.]*\).*/\1/')
L_60PCT=$(echo "$LEFT_STATS" | sed 's/.*60fps%=\([0-9.]*\).*/\1/')
L_STREAK=$(echo "$LEFT_STATS" | sed 's/.*max_streak=\([0-9]*\).*/\1/')
L_P50_US=$(echo "$LEFT_INTERVALS" | sed 's/.*p50=\([0-9]*\)us.*/\1/')
L_P95_US=$(echo "$LEFT_INTERVALS" | sed 's/.*p95=\([0-9]*\)us.*/\1/')
L_P99_US=$(echo "$LEFT_INTERVALS" | sed 's/.*p99=\([0-9]*\)us.*/\1/')
L_P50=$(awk "BEGIN {printf \"%.1f\", $L_P50_US / 1000}")
L_P95=$(awk "BEGIN {printf \"%.1f\", $L_P95_US / 1000}")
L_P99=$(awk "BEGIN {printf \"%.1f\", $L_P99_US / 1000}")

# Parse RIGHT
R_FRAMES=$(echo "$RIGHT_STATS" | sed 's/.*frames=\([0-9]*\).*/\1/')
R_DURATION=$(echo "$RIGHT_STATS" | sed 's/.*duration=\([0-9.]*\)s.*/\1/')
R_FPS=$(echo "$RIGHT_STATS" | sed 's/.*avg_fps=\([0-9.]*\).*/\1/')
R_60PCT=$(echo "$RIGHT_STATS" | sed 's/.*60fps%=\([0-9.]*\).*/\1/')
R_STREAK=$(echo "$RIGHT_STATS" | sed 's/.*max_streak=\([0-9]*\).*/\1/')
R_P50_US=$(echo "$RIGHT_INTERVALS" | sed 's/.*p50=\([0-9]*\)us.*/\1/')
R_P95_US=$(echo "$RIGHT_INTERVALS" | sed 's/.*p95=\([0-9]*\)us.*/\1/')
R_P99_US=$(echo "$RIGHT_INTERVALS" | sed 's/.*p99=\([0-9]*\)us.*/\1/')
R_P50=$(awk "BEGIN {printf \"%.1f\", $R_P50_US / 1000}")
R_P95=$(awk "BEGIN {printf \"%.1f\", $R_P95_US / 1000}")
R_P99=$(awk "BEGIN {printf \"%.1f\", $R_P99_US / 1000}")

printf "%-6s %6s fps | %5s%% at 60fps | streak: %4s | p50: %6sms | p95: %6sms | p99: %6sms\n" \
    "LEFT:" "$L_FPS" "$L_60PCT" "$L_STREAK" "$L_P50" "$L_P95" "$L_P99"
printf "%-6s %6s fps | %5s%% at 60fps | streak: %4s | p50: %6sms | p95: %6sms | p99: %6sms\n" \
    "RIGHT:" "$R_FPS" "$R_60PCT" "$R_STREAK" "$R_P50" "$R_P95" "$R_P99"

echo ""
echo "LEFT:  ${L_FRAMES} frames over ${L_DURATION}s"
echo "RIGHT: ${R_FRAMES} frames over ${R_DURATION}s"
echo ""
echo "Log: $LOG"
