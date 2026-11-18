#!/usr/bin/env bash
# PUBLIC GUARD: fails if a repository's tracked files contain agentic-process material. Used before every push to a public repo and in each public repo's CI.
# Usage: public-guard.sh [DIR]   (DIR defaults to the current directory; checks files tracked by git in DIR, or all files if DIR is not a git repo)
# Two tests: (1) FILE NAMES that are harness/process files; (2) CONTENT that references them. This script and its CI workflow are exempt from the content test.
set -u
DIR="${1:-.}"; cd "$DIR" || exit 2
NAMES='(^|/)(CLAUDE|AGENTS|GEMINI)\.md$|(^|/)\.claude(/|$)|(^|/)(SESSION_STATE|MILESTONES[0-9]*|BRIEFING|HIRE-DRAFT)\.md$|(^|/)BUILD-QUEUE|ORDER-TEMPLATE|(^|/)prompts-out(/|$)|(^|/)instructions-notes|(^|/)AI-LOG\.md$|(^|/)DECISION_OUTCOMES\.md$|(^|/)STAGES\.md$|PLAN_TEMPLATE|(^|/)docs/prompts(/|$)|(^|/)memory/MEMORY\.md$'
CONTENT='CLAUDE\.md|AGENTS\.md|SESSION_STATE|MILESTONES[0-9]*\.md|BUILD-QUEUE|ORDER-TEMPLATE|prompts-out|stage orders?|context[- ]persistence|long-term memory (method|system) of|decision outcome log'
EXEMPT='(^|/)(public-guard\.sh|no-process-files\.yml|\.gitignore)$'
if git rev-parse --git-dir >/dev/null 2>&1; then files=$(git ls-files); else files=$(find . -type f -not -path './.git/*' | sed 's|^\./||'); fi
bad_names=$(printf '%s\n' "$files" | grep -E "$NAMES" | grep -v -E "$EXEMPT")
bad_content=""
while IFS= read -r f; do
  [ -f "$f" ] || continue
  printf '%s' "$f" | grep -q -E "$EXEMPT" && continue
  case "$f" in *.png|*.jpg|*.jpeg|*.gif|*.ico|*.pdf|*.woff*|*.gz|*.zip|*.tar|*.wasm|*.bin) continue;; esac
  if grep -I -q -i -E "$CONTENT" "$f" 2>/dev/null; then bad_content="$bad_content$f\n"; fi
done <<< "$files"
rc=0
if [ -n "$bad_names" ]; then echo "PUBLIC GUARD: forbidden process files are tracked:"; printf '%s\n' "$bad_names" | sed 's/^/  /'; rc=1; fi
if [ -n "$bad_content" ]; then echo "PUBLIC GUARD: files reference agentic-process material:"; printf "$bad_content" | sed 's/^/  /'; rc=1; fi
[ $rc -eq 0 ] && echo "public guard: clean"
exit $rc
