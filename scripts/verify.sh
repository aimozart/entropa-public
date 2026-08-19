#!/usr/bin/env bash
# The one canonical verification entrypoint for this repo — named obviously so a human
# or automated reviewer finds it without piecing together scattered docs. Runs the exact
# same checks as .github/workflows/ci.yml, locally.
set -euo pipefail

echo "== cargo fmt --check =="
cargo fmt --all -- --check

echo "== cargo clippy -D warnings =="
cargo clippy --workspace --all-targets -- -D warnings

echo "== cargo test --workspace =="
cargo test --workspace

echo "== gitleaks (secret scan) =="
./scripts/security/secret-scan.sh

echo "== cargo audit (dependency scan) =="
./scripts/security/dependency-audit.sh

echo ""
echo "ALL CHECKS PASSED — this repo's CI is green."
