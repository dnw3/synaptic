#!/usr/bin/env bash
set -euo pipefail
# Prevent OpenAI wrapper pattern from proliferating outside synaptic-openai.
# Scans all crate src/ directories for the combination of:
#   1. importing synaptic_openai
#   2. implementing ChatModel for a local type
# Allowlist: crates that genuinely implement their own ChatModel.
ALLOW="synaptic-openai|synaptic-anthropic|synaptic-gemini|synaptic-ollama|synaptic-bedrock"
FAIL=0
for crate_dir in crates/synaptic-*/; do
  crate_name=$(basename "$crate_dir")
  [[ "$crate_name" =~ ^($ALLOW)$ ]] && continue
  src_dir="$crate_dir/src"
  [[ -d "$src_dir" ]] || continue
  if rg -q 'use synaptic_openai' "$src_dir" 2>/dev/null && \
     rg -q 'impl ChatModel for' "$src_dir" 2>/dev/null; then
    echo "ERROR: OpenAI wrapper delegation detected in $crate_dir"
    FAIL=1
  fi
done
exit $FAIL
