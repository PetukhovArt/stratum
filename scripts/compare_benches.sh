#!/usr/bin/env bash
# Compare two criterion baselines stored under target/criterion/<bench>/<baseline>.
# Fails (exit 1) when any bench regresses more than the threshold.
set -euo pipefail

BASE="${1:-main}"
NEW="${2:-pr}"
THRESHOLD="${BENCH_THRESHOLD_PCT:-10}"

BASELINE_DIR="target/criterion"
if [ ! -d "$BASELINE_DIR" ]; then
  echo "::warning::no criterion baseline dir at $BASELINE_DIR — skipping compare"
  exit 0
fi

regressions=0
while IFS= read -r -d '' base_json; do
  bench_dir="$(dirname "$(dirname "$base_json")")"
  bench_name="$(basename "$bench_dir")"
  new_json="$bench_dir/$NEW/estimates.json"
  if [ ! -f "$new_json" ]; then
    echo "skip: $bench_name (no $NEW baseline)"
    continue
  fi
  base_mean=$(python3 -c 'import json,sys;print(json.load(open(sys.argv[1]))["mean"]["point_estimate"])' "$base_json")
  new_mean=$(python3 -c 'import json,sys;print(json.load(open(sys.argv[1]))["mean"]["point_estimate"])' "$new_json")
  delta_pct=$(python3 -c "b=$base_mean;n=$new_mean;print(((n-b)/b)*100)")
  printf '%-60s base=%.2fns new=%.2fns delta=%+.2f%%\n' "$bench_name" "$base_mean" "$new_mean" "$delta_pct"
  is_regression=$(python3 -c "print(1 if $delta_pct > $THRESHOLD else 0)")
  if [ "$is_regression" = "1" ]; then
    echo "::error::$bench_name regressed by ${delta_pct}% (threshold ${THRESHOLD}%)"
    regressions=$((regressions+1))
  fi
done < <(find "$BASELINE_DIR" -path "*/$BASE/estimates.json" -print0)

if [ "$regressions" -gt 0 ]; then
  echo "$regressions bench(es) regressed beyond ${THRESHOLD}%"
  exit 1
fi
echo "no regressions"
