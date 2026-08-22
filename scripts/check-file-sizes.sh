#!/usr/bin/env bash
# Enforces CLAUDE.md L20: source files stay under a 500-line soft ceiling, so growth
# gets caught here — automatically, every run — instead of waiting for a builder
# report or a human to notice a file crept past it (exactly what happened to
# persistence.rs, 2026-08-22: grew to 784 lines across several same-session changes
# before anyone caught it). This is the "automated check and balance" half of L20;
# the other half (actually splitting an offending file) still needs a human/agent
# judgment call about how to group by responsibility.
set -euo pipefail

LIMIT=500
offenders=()

while IFS= read -r -d '' file; do
    lines=$(wc -l < "$file")
    if [ "$lines" -gt "$LIMIT" ]; then
        offenders+=("$file: $lines lines")
    fi
done < <(find crates -name "*.rs" -print0)

if [ "${#offenders[@]}" -gt 0 ]; then
    echo "L20 violation: the following file(s) exceed the ${LIMIT}-line soft ceiling:"
    printf '  %s\n' "${offenders[@]}"
    echo ""
    echo "Extract into a submodule grouped by responsibility (see routes/*.rs for the"
    echo "established shape), co-locating each piece's own tests, not just appending."
    exit 1
fi

echo "All source files under the ${LIMIT}-line ceiling."
