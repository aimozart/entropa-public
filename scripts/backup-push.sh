#!/usr/bin/env bash
# Daily automated backup push: entropa-public -> Gitea mirror (entropa_chain_public_BU).
# GitHub (origin) remains the canonical public home; this is redundancy only.
set -euo pipefail

REPO_DIR="/home/sbaker/entropa-public"
LOG_FILE="/home/sbaker/entropa-public/backup-push.log"

cd "$REPO_DIR"
{
  echo "=== $(date -u +%Y-%m-%dT%H:%M:%SZ) ==="
  git fetch origin main --quiet
  git push backup main
  echo "Backup push succeeded."
} >> "$LOG_FILE" 2>&1
