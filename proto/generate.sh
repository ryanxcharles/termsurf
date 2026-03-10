#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."

# Generate C code from the proto schema.
protoc-c --c_out=ghostboard/src/protobuf --proto_path=proto proto/termsurf.proto

echo "Generated ghostboard/src/protobuf/termsurf.pb-c.{c,h}"
