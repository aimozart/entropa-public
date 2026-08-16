#!/usr/bin/env bash
set -euo pipefail

echo "Running gitleaks against committed git history..."
gitleaks detect --source .

echo "Secret scan passed."
