#!/usr/bin/env bash
#
# Publish all synaptic crates to crates.io in dependency order.
#
# Usage:
#   ./scripts/publish.sh          # publish all crates
#   ./scripts/publish.sh --dry-run # dry-run (no actual publish)
#
# Notes:
#   - Uses --no-verify because workspace crates have circular dev-dependencies
#     (e.g., synaptic-macros <-> synaptic-middleware) that can't all be on
#     crates.io simultaneously during first publish.
#   - Workspace tests (cargo test --workspace) verify everything builds correctly.
#
set -euo pipefail

DRY_RUN=""
if [[ "${1:-}" == "--dry-run" ]]; then
  DRY_RUN="--dry-run"
  echo "==> DRY RUN mode"
fi

# Topological publish order (runtime dependencies before dependents)
# Level 0: no internal deps
# Level 1: depends on core only
# Level 2: depends on level-1 crates
# Level 3: depends on level-2 crates
# Level 4: facade (depends on all)
CRATES=(
  # Level 0
  synaptic-core
  synaptic-macros
  # Level 1 (depend on core only)
  synaptic-runnables
  synaptic-store
  synaptic-middleware
  synaptic-models
  synaptic-callbacks
  synaptic-mcp
  synaptic-embeddings
  synaptic-memory
  synaptic-parsers
  synaptic-prompts
  synaptic-loaders
  synaptic-splitters
  # Level 2 (depend on level-1 crates)
  synaptic-tools
  synaptic-retrieval
  synaptic-cache
  synaptic-eval
  synaptic-vectorstores
  # Level 3
  synaptic-graph
  synaptic-deep
  # Level 4 (facade)
  synaptic
)

TOTAL=${#CRATES[@]}
IDX=0

for crate in "${CRATES[@]}"; do
  IDX=$((IDX + 1))
  echo ""
  echo "==> [$IDX/$TOTAL] Publishing $crate ..."
  cargo publish -p "$crate" $DRY_RUN --allow-dirty --no-verify

  if [[ -z "$DRY_RUN" ]]; then
    # Wait for crates.io index to update before publishing dependents
    echo "    Waiting 30s for crates.io index..."
    sleep 30
  fi
done

echo ""
echo "==> All $TOTAL crates published successfully!"
