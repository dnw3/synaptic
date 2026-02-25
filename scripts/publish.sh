#!/usr/bin/env bash
set -euo pipefail
#
# Publish all synaptic crates to crates.io in dependency order.
#
# Uses `cargo metadata` + jq for true DAG topological sort instead of
# a hardcoded crate list.
#
# Handles circular dev-dependencies (e.g., synaptic-macros <-> synaptic-middleware)
# by temporarily stripping unpublished workspace dev-deps before packaging.
#
# Usage:
#   ./scripts/publish.sh          # publish all crates
#   ./scripts/publish.sh --dry-run # dry-run (no actual publish)
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
PUBLISHED=""
IDX=0

for crate in $CRATES; do
  IDX=$((IDX + 1))
  echo "==> [$IDX/$TOTAL] Publishing $crate ..."

  # Find the manifest path for this crate
  MANIFEST=$(cargo metadata --no-deps --format-version 1 \
    | jq -r ".packages[] | select(.name == \"$crate\") | .manifest_path")
  MANIFEST_DIR=$(dirname "$MANIFEST")

  # Get this crate's dev-deps that are workspace crates not yet published
  UNPUBLISHED_DEV_DEPS=$(cargo metadata --no-deps --format-version 1 \
    | jq -r --arg crate "$crate" --arg published "$PUBLISHED" '
      ($published | split("\n") | map(select(. != ""))) as $pub
      | .packages[] | select(.name == $crate)
      | .dependencies[]
      | select(.kind == "dev")
      | select(.path != null)
      | select(.name as $n | $pub | index($n) | not)
      | .name' 2>/dev/null || true)

  PATCHED=false
  if [[ -n "$UNPUBLISHED_DEV_DEPS" ]]; then
    echo "    Patching: stripping unpublished dev-deps: $(echo $UNPUBLISHED_DEV_DEPS | tr '\n' ' ')"
    cp "$MANIFEST" "${MANIFEST}.bak"
    PATCHED=true
    for dep in $UNPUBLISHED_DEV_DEPS; do
      # Remove line matching "dep-name = " in [dev-dependencies] section
      # Use awk to only remove within [dev-dependencies] block
      awk -v dep="$dep" '
        /^\[dev-dependencies\]/ { in_dev=1; print; next }
        /^\[/ { in_dev=0; print; next }
        in_dev && $0 ~ "^"dep" *=" { next }
        { print }
      ' "$MANIFEST" > "${MANIFEST}.tmp"
      mv "${MANIFEST}.tmp" "$MANIFEST"
    done
  fi

  set +e
  OUTPUT=$(cargo publish -p "$crate" $DRY_RUN --allow-dirty --no-verify 2>&1)
  EXIT_CODE=$?
  set -e
  echo "$OUTPUT"
  if [[ $EXIT_CODE -ne 0 ]]; then
    if echo "$OUTPUT" | grep -q "already exists\|already uploaded"; then
      echo "    Already published, skipping."
    else
      echo "    ERROR: Failed to publish $crate"
      exit 1
    fi
  fi

  # Restore original Cargo.toml if patched
  if [[ "$PATCHED" == "true" ]]; then
    mv "${MANIFEST}.bak" "$MANIFEST"
    echo "    Restored Cargo.toml"
  fi

  PUBLISHED="${PUBLISHED}${crate}\n"
  [[ -z "$DRY_RUN" ]] && echo "    Waiting 30s..." && sleep 30
done
echo "==> All $TOTAL crates published!"
