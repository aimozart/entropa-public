#!/usr/bin/env bash
# Post-deploy smoke test for Entropa. Run this after every deploy, not just when something
# looks broken -- two of today's three incidents (2026-08-12) would have been caught
# immediately by this instead of relying on someone noticing the explorer was slow/broken.
set -uo pipefail

HOST="${1:-https://entropa.space}"
FAIL=0

check() {
  local name="$1" url="$2" max_bytes="$3" max_seconds="$4"
  local out
  out=$(curl -s -o /dev/null -w "%{http_code} %{size_download} %{time_total}" "$url")
  local code size time
  read -r code size time <<< "$out"

  if [ "$code" != "200" ]; then
    echo "FAIL  $name: HTTP $code (expected 200) -- $url"
    FAIL=1
    return
  fi
  if [ "$size" -gt "$max_bytes" ]; then
    echo "FAIL  $name: ${size} bytes (max ${max_bytes}) -- too large for a live-refreshing page"
    FAIL=1
    return
  fi
  if awk -v t="$time" -v m="$max_seconds" 'BEGIN { exit !(t > m) }'; then
    echo "FAIL  $name: ${time}s (max ${max_seconds}s) -- too slow"
    FAIL=1
    return
  fi
  echo "OK    $name: HTTP $code, ${size} bytes, ${time}s"
}

check "health"          "$HOST/api/health"          5000      1.0
check "chain (default explorer/flow request)" "$HOST/api/chain?limit=50" 1500000 1.5
check "chain (unbounded default)" "$HOST/api/chain" 15000000 5.0
check "explorer page"   "$HOST/"                     200000    1.0
check "flow page"       "$HOST/flow"                 200000    1.0

if [ "$FAIL" -eq 1 ]; then
  echo
  echo "SMOKE TEST FAILED -- do not consider this deploy done."
  exit 1
fi
echo
echo "Smoke test passed."
