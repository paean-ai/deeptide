#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)

echo "==> Building ds4"
make -C "$ROOT/native/ds4"

echo "==> Building dsgo"
swift build --package-path "$ROOT/native/dsgo"

echo "==> Native build complete"
