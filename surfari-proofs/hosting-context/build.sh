#!/bin/sh
set -eu

cd "$(dirname "$0")"
mkdir -p build

clang \
  -fobjc-arc \
  -ObjC \
  -Wall \
  -Wextra \
  -Werror \
  -framework Cocoa \
  -framework WebKit \
  -framework QuartzCore \
  WebKitHostingProof.m \
  -o build/WebKitHostingProof

printf '%s\n' "built surfari-proofs/hosting-context/build/WebKitHostingProof"
