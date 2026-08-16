#!/usr/bin/env bash
set -euo pipefail

echo "Running cargo audit against Cargo.lock..."
cargo audit

echo "Dependency audit passed."
