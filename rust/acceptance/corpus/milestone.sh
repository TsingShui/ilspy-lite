#!/bin/sh
# Copyright (c) 2026 Tsingshui
# SPDX-License-Identifier: MIT
#
# End-of-slice corpus milestone: build, rescan, compare, gate, bucket.
#
# Usage: milestone.sh <scan-output-dir> <previous-baseline.json> [<second-previous-baseline.json>]
#
# The scan always runs the reference selection corpus.py now applies: one deterministic
# file per declared identity plus the single hop of references those files declare
# (corpus.py baseline --references declared --reference-ambiguity tie-break
# --reference-hop 1). Set MILESTONE_SKIP_BUILD=1 to reuse the release binary that is
# already present instead of rebuilding it.
#
# Every printed table is derived from the scan just written and from the previous
# baselines named on the command line. Exit status is 0 only when the scan is complete
# and no image lost a method relative to a previous baseline, 1 for a lost method or an
# incomplete scan, and 2 for a usage or input problem.
set -eu

usage() {
    echo "usage: $0 <scan-output-dir> <previous-baseline.json> [<second-previous-baseline.json>]" >&2
    exit 2
}

[ "$#" -ge 2 ] || usage
SCAN_DIR=$1
shift

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd)
INVENTORY="$ROOT/rust/report/core-type-roots/scope-policy/implementation-inventory.json"
CLI="$ROOT/rust/target/release/ilspy-lite"
CORPUS="$ROOT/rust/acceptance/corpus/corpus.py"
COMPARE="$ROOT/rust/report/compare-recovery-scans.py"
GATE="$ROOT/rust/report/per-assembly-gate.py"
BASELINE="$SCAN_DIR/baseline.json"

command -v python3 >/dev/null 2>&1 || { echo "milestone: python3 is required" >&2; exit 2; }
[ -f "$INVENTORY" ] || { echo "milestone: frozen inventory missing: $INVENTORY" >&2; exit 2; }
[ -f "$CORPUS" ] || { echo "milestone: corpus.py missing: $CORPUS" >&2; exit 2; }
[ -f "$COMPARE" ] || { echo "milestone: compare-recovery-scans.py missing: $COMPARE" >&2; exit 2; }
[ -f "$GATE" ] || { echo "milestone: per-assembly-gate.py missing: $GATE" >&2; exit 2; }
for previous in "$@"; do
    [ -f "$previous" ] || { echo "milestone: previous baseline missing: $previous" >&2; exit 2; }
done

echo "== build release"
if [ "${MILESTONE_SKIP_BUILD:-0}" = "1" ]; then
    echo "skipped (MILESTONE_SKIP_BUILD=1)"
else
    cargo build --manifest-path "$ROOT/rust/Cargo.toml" --release --locked --offline
fi
[ -x "$CLI" ] || { echo "milestone: release CLI missing after build: $CLI" >&2; exit 1; }

echo "== scan"
scan_status=0
python3 "$CORPUS" baseline --inventory "$INVENTORY" --cli "$CLI" --output "$SCAN_DIR" \
    --references declared --reference-ambiguity tie-break --reference-hop 1 || scan_status=$?
case $scan_status in
    0|2) ;;
    *) echo "milestone: scan did not complete (exit $scan_status)" >&2; exit 1 ;;
esac

echo "== summary"
python3 - "$BASELINE" <<'PY'
import json
import sys

baseline = json.loads(open(sys.argv[1]).read())
print(json.dumps({"references": baseline.get("references"), "summary": baseline["summary"],
                  "elapsed_ms": baseline["elapsed_ms"]}))
PY

compared=""
for previous in "$@"; do
    name=$(basename "$(dirname "$previous")")
    result="$SCAN_DIR/compare-vs-$name.json"
    echo "== compare vs $previous"
    python3 "$COMPARE" "$previous" "$BASELINE" "$result"
    compared="$compared $result"
    python3 - "$result" <<'PY'
import json
import sys

comparison = json.loads(open(sys.argv[1]).read())
print(json.dumps({"before": comparison["before"], "after": comparison["after"],
                  "gained_methods": comparison["newly_recovered"],
                  "gained_images": comparison["gained_images"],
                  "lost_methods": len(comparison["regressions"]),
                  "incomplete_before": comparison["before"]["incomplete_images"],
                  "incomplete_after": comparison["after"]["incomplete_images"]}))
PY
done

echo "== per-assembly gate"
gate_status=0
python3 "$GATE" "$BASELINE" || gate_status=$?
case $gate_status in
    0) ;;
    1) echo "(per-assembly gate is unsatisfied; the table above is the remaining work)" ;;
    *) echo "milestone: gate table failed (exit $gate_status)" >&2; exit 1 ;;
esac

echo "== top 10 first blockers over body images"
python3 - "$BASELINE" <<'PY'
import collections
import json
import sys

baseline = json.loads(open(sys.argv[1]).read())
reasons = collections.Counter()
for image in baseline["images"]:
    if image["bodies"]:
        reasons.update(image["reasons"])
total = sum(reasons.values())
for reason, count in sorted(reasons.items(), key=lambda item: (-item[1], item[0]))[:10]:
    print(f"{count:8d} {100 * count / total:6.2f}% {reason}")
print(f"{total:8d} 100.00% unrecovered bodies over body images")
PY

echo "== verdict"
verdict=""
for result in $compared; do
    verdict="$verdict $result"
done
python3 - "$BASELINE" $verdict <<'PY'
import json
import sys

baseline = json.loads(open(sys.argv[1]).read())
incomplete = [image["assembly"] for image in baseline["images"] if not image["scan_complete"]]
lost = {}
for path in sys.argv[2:]:
    for regression in json.loads(open(path).read())["regressions"]:
        lost.setdefault(regression["assembly"], 0)
        lost[regression["assembly"]] += 1
for name in sorted(lost):
    print(f"lost {lost[name]} methods in {name}")
for name in sorted(incomplete):
    print(f"incomplete scan of {name}")
print(json.dumps({"complete": not incomplete, "lost_methods": sum(lost.values()),
                  "incomplete_images": len(incomplete)}))
sys.exit(1 if incomplete or lost else 0)
PY
