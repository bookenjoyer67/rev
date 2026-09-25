#!/usr/bin/env bash
# Run 003 harness — komun-contract-auditor v0.1.2, identical task string to Runs 001/002.
#
# The only intended differences from Runs 001/002:
#   1. the agent definition (v0.1.2 is the single variable under test),
#   2. the lab artifacts that hold this workflow's answers and hindsight are moved OUT of the
#      mounted workspace for the duration of the run (docs/contract-audit/truth.md says to do this),
#   3. they are restored immediately afterwards, and the restored tree is compared with git.
set -u
W=/home/computing/.hermes/profiles/dev/cache/scratch/run-003
REPO=/home/computing/rev
LAB="docs/iteration-log.md docs/agent-rubric.md docs/prd.md docs/rubric.md docs/contract-audit"

mkdir -p "$W/moved"
cd "$REPO"

echo "=== 0. tree state before anything moves ==="
git status --porcelain | tee "$W/git-status-before.txt"
git rev-parse HEAD | tee "$W/head-before.txt"

echo "=== 1. move the answer key and the hindsight out of the mount ==="
for p in $LAB; do
  mkdir -p "$W/moved/$(dirname "$p")"
  mv "$p" "$W/moved/$p"
done
echo "moved: $LAB"
git status --porcelain | tee "$W/git-status-after-move.txt"

echo "=== 2. fresh container (same recipe as Runs 001/002) ==="
docker rm -f agent-rev >/dev/null 2>&1
REPO=$REPO sandbox/run-agent.sh 2>&1 | grep -E 'workspace|container|broker|health'

echo "=== 3. graded run ==="
START_HOST=$(date '+%Y-%m-%dT%H:%M:%S%:z'); T0=$(date +%s)
echo "RUN 003 START $START_HOST"
docker exec -w /workspace agent-rev claude -p --agent komun-contract-auditor \
  "Audit AGENTS.md against the repository and report the result." > "$W/run-003-output.txt" 2>&1
echo "exit=$?"
T1=$(date +%s)
END_HOST=$(date '+%Y-%m-%dT%H:%M:%S%:z')
echo "RUN 003 END $END_HOST elapsed=$((T1-T0))s" | tee "$W/timing.txt"
wc -c "$W/run-003-output.txt"

echo "=== 4. capture the transcript ==="
SID=$(docker exec agent-rev bash -c 'ls -t /root/.claude/projects/-workspace/*.jsonl 2>/dev/null | head -1')
docker cp "agent-rev:$SID" "$W/run-003-transcript.jsonl"
ls -l "$W/run-003-transcript.jsonl"

echo "=== 5. containment, host-side, before restoring ==="
git status --porcelain | tee "$W/git-status-after-run.txt"
find "$REPO" -newermt "$(cat "$W/timing.txt" | sed -n 's/.*END \([0-9T:-]*\) .*/\1/p')" \
  -not -path '*/target/*' -not -path '*/.git/*' -not -path '*/node_modules/*' | head -10
echo "(anything above besides the moved artifacts is a write by the run)"
ls -la "$REPO/.claude/" | tee "$W/dot-claude-after-run.txt"

echo "=== 6. restore the artifacts ==="
for p in $LAB; do mv "$W/moved/$p" "$REPO/$p"; done
git status --porcelain | tee "$W/git-status-restored.txt"
echo "(empty = the tree is byte-identical to the committed state)"
