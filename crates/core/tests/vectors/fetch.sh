#!/usr/bin/env bash
# Reproduce the COMPLETE official NIST ACVP known-answer test vectors for ML-DSA
# (FIPS 204), straight from NIST's own repository.
#
# The committed `ml_dsa_65_acvp.json` is a representative subset (kept small so CI
# stays fast). This script fetches the full upstream files so anyone can independently
# verify Entropa against every official vector, not just the ones we committed.
#
# Usage:  ./fetch.sh [output-dir]     (default: ./full)
set -euo pipefail

BASE="https://raw.githubusercontent.com/usnistgov/ACVP-Server/master/gen-val/json-files"
OUT="${1:-$(dirname "$0")/full}"
mkdir -p "$OUT"

for f in ML-DSA-keyGen-FIPS204 ML-DSA-sigGen-FIPS204 ML-DSA-sigVer-FIPS204; do
  echo "fetching $f ..."
  curl -fsSL "$BASE/$f/internalProjection.json" -o "$OUT/${f}.json"
done

echo
echo "Fetched into $OUT:"
ls -lh "$OUT"
echo
echo "These are NIST's files, unmodified. The committed subset in"
echo "ml_dsa_65_acvp.json was extracted from the first two, verbatim."
