#!/usr/bin/env bash
# dispatch.sh <task-id> <machine: chiz|mini>
# Preflights the model server, makes a worktree on branch task/<id>, runs opencode headless
# in it with tasks/<id>-*.md as the prompt, and logs to logs/<id>.log.
# opencode reads provider credentials from ~/.config/opencode; none live in this repo.
# Set CHIZ_LLM_API_KEY to also verify chiz's loaded model alias (its /props needs the key).
set -euo pipefail
id=$1; machine=$2
root=$(git rev-parse --show-toplevel)
case $machine in
  chiz) model=chiz/qwen3.8-27b; base=http://100.66.187.40:8100; alias=qwen3.8-27b
        key=${CHIZ_LLM_API_KEY:-} ;;
  mini) model=llamacpp/qwen3.6-35b-a3b-uncensored; base=http://100.87.236.63:11435
        alias=qwen3.6-35b-a3b-uncensored; key=none ;;
  *) echo "unknown machine $machine" >&2; exit 2 ;;
esac
model=${MODEL_OVERRIDE:-$model}
[ "$(curl -s -m 5 -o /dev/null -w '%{http_code}' "$base/health")" = 200 ] || { echo "$machine down" >&2; exit 3; }
if [ -n "$key" ]; then
  live=$(curl -s -m 5 -H "Authorization: Bearer $key" "$base/props" \
    | python3 -c 'import sys,json;print(json.load(sys.stdin).get("model_alias"))')
  [ "$live" = "$alias" ] || { echo "$machine serves $live, expected $alias" >&2; exit 4; }
fi
spec=$(ls "$root"/tasks/"$id"-*.md | head -1)
wt="$root/../kc-wt/$id"
if [ -d "$wt" ]; then
  # A reused worktree must pick up everything merged since it was made (config, deps).
  [ -z "$(git -C "$wt" status --porcelain)" ] && git -C "$wt" merge -q --no-edit main
else
  git -C "$root" worktree add -q -b "task/$id" "$wt" main
fi
mkdir -p "$root/logs"
log="$root/logs/$id.log"
echo "[$(date +%T)] $id → $machine ($model) in $wt" | tee "$log"
cd "$wt"
start=$(date +%s)
OPENCODE_EXPERIMENTAL_OUTPUT_TOKEN_MAX=100000 timeout "${TASK_TIMEOUT:-1800}" opencode run -m "$model" "$(cat "$spec")" >>"$log" 2>&1 || echo "[exit $?]" >>"$log"
echo "[$(date +%T)] $id finished in $(( $(date +%s) - start ))s" | tee -a "$log"
# A run that ends with nothing changed usually means a reply was cut off mid-reasoning.
if [ -z "$(git status --porcelain)" ] && [ "$(git rev-list --count main..HEAD)" = 0 ]; then
  echo "[$(date +%T)] $id [NO CHANGES]" | tee -a "$log"
fi
