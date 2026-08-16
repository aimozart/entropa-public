#!/usr/bin/env bash
set -euo pipefail

echo "Running gitleaks against committed git history..."
gitleaks detect --source . --verbose --redact

echo "Secret scan passed."
