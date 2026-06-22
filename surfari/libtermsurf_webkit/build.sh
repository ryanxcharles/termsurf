#!/bin/sh
set -eu

cd "$(dirname "$0")"

repo_root="$(git rev-parse --show-toplevel)"
webkit_build="$repo_root/webkit/src/WebKitBuild/Debug"

if [ ! -d "$webkit_build/WebKit.framework" ]; then
  printf '%s\n' "error: missing $webkit_build/WebKit.framework" >&2
  printf '%s\n' "run: webkit/src/Tools/Scripts/build-webkit --debug" >&2
  exit 1
fi

mkdir -p build

common_flags="
  -fobjc-arc
  -Wall
  -Wextra
  -Werror
  -Wno-deprecated-declarations
  -Iinclude
  -F$webkit_build
"

common_links="
  -framework Cocoa
  -framework QuartzCore
  -framework WebKit
  -rpath $webkit_build
"

clang++ \
  $common_flags \
  -std=c++17 \
  -dynamiclib \
  -install_name @rpath/libtermsurf_webkit.dylib \
  src/libtermsurf_webkit.mm \
  $common_links \
  -o build/libtermsurf_webkit.dylib

install_name_tool \
  -change /System/Library/Frameworks/WebKit.framework/Versions/A/WebKit \
  @rpath/WebKit.framework/Versions/A/WebKit \
  build/libtermsurf_webkit.dylib

clang \
  -Wall \
  -Wextra \
  -Werror \
  -Iinclude \
  smoke-test/smoke_test.c \
  -Lbuild \
  -ltermsurf_webkit \
  -rpath "$PWD/build" \
  -rpath "$webkit_build" \
  -o build/smoke-test

printf '%s\n' "built surfari/libtermsurf_webkit/build/libtermsurf_webkit.dylib"
printf '%s\n' "built surfari/libtermsurf_webkit/build/smoke-test"
