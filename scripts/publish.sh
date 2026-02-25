#!/usr/bin/env bash
set -euo pipefail
#
# Publish all synaptic crates to crates.io in dependency order.
#
# Uses `cargo metadata` + jq for true DAG topological sort instead of
# a hardcoded crate list.
#
# Usage:
#   ./scripts/publish.sh          # publish all crates
#   ./scripts/publish.sh --dry-run # dry-run (no actual publish)
#
# Notes:
#   - Uses --no-verify because workspace crates have circular dev-dependencies
#     (e.g., synaptic-macros <-> synaptic-middleware) that can't all be on
#     crates.io simultaneously during first publish.
#
DRY_RUN=""
[[ "${1:-}" == "--dry-run" ]] && DRY_RUN="--dry-run" && echo "==> DRY RUN mode"

# Extract publishable crates from crates/ directory and topologically sort by dependencies
CRATES=$(cargo metadata --no-deps --format-version 1 \
  | jq -r '
    # Step 1: Filter to crates/ directory packages that are publishable
    [.packages[]
     | select(.source == null)
     | select(.manifest_path | test("/crates/"))
     | select(.publish == null or (.publish | length > 0))
    ] as $pkgs
    | ($pkgs | map(.name) | sort) as $names
    # Step 2: Build internal dependency graph (normal deps only, skip dev/build)
    | [.packages[]
       | select(.name as $n | $names | index($n))
       | {name, internal_deps: [.dependencies[] | select(.kind == null) | .name | select(. as $d | $names | index($d))]}
      ] as $graph
    # Step 3: Kahn topological sort
    | def topo_sort:
        . as $g
        | ($g | map({(.name): .internal_deps}) | add) as $adj
        | ($g | map(.name)) as $all
        | ($g | map(select(.internal_deps | length == 0) | .name)) as $ready
        | {queue: $ready, result: [], remaining: ($all - $ready), adj: $adj}
        | until(.queue | length == 0;
            .queue[0] as $cur
            | .result += [$cur]
            | .queue |= .[1:]
            | .remaining as $rem
            | reduce ($rem[]) as $n (.;
                if (.adj[$n] - .result) | length == 0
                then .queue += [$n] | .remaining -= [$n]
                else . end))
        | .result;
    $graph | topo_sort | .[]')

TOTAL=$(echo "$CRATES" | wc -l | tr -d ' ')
IDX=0
for crate in $CRATES; do
  IDX=$((IDX + 1))
  echo "==> [$IDX/$TOTAL] Publishing $crate ..."
  cargo publish -p "$crate" $DRY_RUN --allow-dirty --no-verify
  [[ -z "$DRY_RUN" ]] && echo "    Waiting 30s..." && sleep 30
done
echo "==> All $TOTAL crates published!"
