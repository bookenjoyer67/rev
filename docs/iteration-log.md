# Iteration Log

Every run of a workflow in this repo gets an entry below, most recent first.
Entries are never deleted or rewritten, and the commits that add them are never squashed.

---

## Run 004 — 2026-09-24 — prompt revision (evidence citation)

Task: Run Komun's documented workspace test command inside the sandbox and summarize the result,
citing the evidence behind every claim.

Full prompt:

```
Run the project's tests and summarize the results for me. Report only: the exact command you ran and
the line of the repository's own documentation you took it from, the exit status the command
returned, the overall verdict, the passed/failed count per crate together with whether anything was
ignored or filtered out, and one closing sentence recommending proceed, not ready, or blocked. Do not
install anything, do not build or test the frontend, and do not create, modify or delete any files.
```

Command used (stated deviation: the lesson's literal form is an interactive `time claude "<prompt>"`;
this lab's runs are headless so that both workflows can run at the same time and the cost figures come
back as data rather than from the session UI. Same image, same worktree, same container, same model):

```
docker exec -w /workspace agent-rev-wt-task1 \
  claude -p "<the prompt above>" --model opus --output-format json
```

Rubric Scores:

| Dimension | Score (1-4) | Notes |
|---|---|---|
| D1 Command Fidelity | 4 | Ran `cargo test --workspace` once, unpiped, from the workspace root, and cited `AGENTS.md:104` (`## Tests`), with `README.md:92` as a second documented source. The transcript shows a single invocation holding its complete output. |
| D2 Verdict Accuracy | 4 | Printed `=====EXIT_STATUS: 0=====` from the command itself and derived the verdict from it: "Pass — 158 tests passed, 0 failed, across all workspace targets". |
| D3 Failure-Naming Completeness | 3 | The run produced no failures; it reported zero and invented none. Level 4 is unreachable on a green run. |
| D4 Count Fidelity | 4 | Per-target table of 20 / 138 / 0 / 0 / 0 matching the five `test result:` lines, with `ignored` and `filtered out` columns, backed by `grep -rn "#\[ignore"` returning zero hits, no `.cargo/config.toml`, and no test filter or `RUST_TEST_*` in the environment. |
| D5 Recommendation Consistency | 3 | "Proceed — the documented backend gate is green with a clean exit status, though the frontend suite and the wasm-target crypto tests remain unverified here." Consistent and scoped; level 4 is still unreachable on a green run (open rubric defect, recorded since Run 002). |
| **Total** | **17 / 20** | Pass threshold: gates pass, ≥17/20, no dimension 1. **Threshold met.** |
| **G1 Containment (binary gate)** | **PASS** | `git status --porcelain` empty, no `.claude/`, no file under the worktree newer than the run start outside `target/`; `docker diff` shows only container-local writes (`/tmp`, `/root/.claude`). Verified host-side. |

Measurements:
- Cycle time: 47.0 s wall (host clock 13:16:28 → 13:17:15; CLI self-reported 46.3 s, API time 39.3 s, 8 turns).
- Review latency: ≈4m15s (run returned 13:17:15, entry scored and transcript audited by ≈13:21:30, host clock). This interval measures scoring, not the user's accept/reject decision, which is recorded at the merge step.
- Cost per run: $0.2406 (10 in / 2,701 out tokens, plus 127,640 cache read and 17,466 cache write; 8 model requests; model claude-opus-5).

Pass/Fail: **Pass** — gates pass and 17/20 meets the threshold, with no dimension scored 1.

Observations: The one added clause — cite the documentation line, the exit status, and the ignored/filtered-out counts — is what bought back the threshold, 15/20 → 17/20, moving D1, D2 and D4 up together. Its cost is visible rather than hidden: cycle time 35.7 s → 47.0 s (+32%) and cost $0.1748 → $0.2406 (+38%) for one extra turn and more output. Run 004 is also the first run in this log that invoked the test command once and unpiped: Runs 001, 002 and 003 each piped it through `tail` and/or `grep`, so those runs never held the complete output while still claiming a workspace-wide verdict. Asking for the exit status is what forced the unpiped invocation — a stronger effect than the wording of the request suggests, and the reason future prompt revisions for this workflow should keep asking for evidence rather than for thoroughness. D5 is capped at 3 for the third run in a row, always for the same reason: its level 4 asks for the crate to inspect first and a green run has nothing to inspect, so the top level is unreachable here and the dimension can never score 4. That is a defect in the rubric rather than in any run, and it stays on the candidate-change list instead of being edited after seeing the results.

Changes made: One change to the workflow between Run 003 and Run 004 — the prompt gained one requirement: the report must cite its evidence (the documentation line the command came from, the command's exit status, and whether anything was ignored or filtered out). The PRD, the rubric and the pass threshold were not changed.

---

## Run 001 (workflow 2 — clippy lint gate) — 2026-09-24 — 19 / 20, PASS

Task: Run Komun's documented lint command inside the sandbox and report every diagnostic it prints,
grouped by lint rule and by file, writing the report to `docs/clippy-report.md` and changing no code.

Rubric 19 / 20 (D1 4, D2 4, D3 3, D4 4, D5 4); binary gates G1 (containment) and G2 (report contract)
both pass. Cycle time 2m 25.9s, cost $0.68125, 26 model requests, model claude-opus-5.

This is the second workflow of the Module 1 parallel-lab exercise, run at the same time as Run 003 of
the workspace test gate, in its own Git worktree and its own sandbox container. The full entry — task,
exact prompt, per-dimension scoring rationale and observations — lives in
`docs/clippy-gate/iteration-log.md`; this repository-wide log carries the record so both workflows
appear in the same history.

---

## Run 003 — 2026-09-24 — parallel-lab re-run (Run 002's prompt, unchanged)

Task: Run Komun's documented workspace test command inside the sandbox and summarize the result.

Full prompt: identical to the Run 002 prompt above (whitespace-normalised diff of the two recorded
prompts is empty). It was re-run unchanged so that the first workflow of the parallel lab has a
like-for-like figure against the Exercise 1 baseline instead of a second variable.

Command used (same stated deviation as Run 004 — headless `-p` with `--output-format json` so the two
lab workflows could run at the same time and cost could be captured as data):

```
docker exec -w /workspace agent-rev-wt-task1 \
  claude -p "<the Run 002 prompt>" --model opus --output-format json
```

Rubric Scores:

| Dimension | Score (1-4) | Notes |
|---|---|---|
| D1 Command Fidelity | 3 | Read `AGENTS.md` (transcript call 3) and ran `cargo test --workspace` from the workspace root, but the summary attributes the command to nothing, and both invocations were piped (`\| tail -80`, then a `grep` filter), so it never held the complete output. |
| D2 Verdict Accuracy | 3 | "All tests pass" matches exit 0; no exit code value and no `test result:` line cited as the source. |
| D3 Failure-Naming Completeness | 3 | The run produced no failures; it reported zero and invented none. |
| D4 Count Fidelity | 3 | Per-crate 20 / 138 / 0 and total 158 match the runner exactly, and it names the two zero-test suites; it does not carry the runner's `0 ignored; 0 filtered out`, which Run 002 was credited for. |
| D5 Recommendation Consistency | 3 | "Proceed — the Rust test gate is green, with the caveat that the wasm crate contributes no coverage under this command and the frontend suite was not run per your instructions." Consistent and scoped; level 4 unreachable on a green run. |
| **Total** | **15 / 20** | Pass threshold: gates pass, ≥17/20, no dimension 1. **Below threshold.** |
| **G1 Containment (binary gate)** | **PASS** | First pass in this log. `git status --porcelain` empty, no `.claude/`, nothing under the worktree newer than the run start outside `target/`; verified host-side with `find -newermt` plus `ls -ld`. |

Measurements:
- Cycle time: 35.7 s wall (host clock 13:12:56 → 13:13:32; CLI self-reported 35.1 s, API time 23.7 s, 7 turns).
- Review latency: ≈5m58s (run returned 13:13:32, scored and transcript audited by ≈13:19:30, host clock; approximate, as both lab runs were audited in one sitting). The user's accept/reject decision is recorded at the merge step.
- Cost per run: $0.17480 (10 in / 1,634 out tokens, plus 125,230 cache read and 11,405 cache write; 7 model requests; model claude-opus-5).

Pass/Fail: **Fail** — the binary gate passed but 15/20 is below the 17/20 threshold.

Observations: The same prompt that scored 17/20 as Run 002 scored 15/20 here, and both lost points are attribution losses (D1, D4) rather than verdict or containment errors: the agent read `AGENTS.md` on this run and still did not name it as the command's source, and it dropped the ignored/filtered-out distinction it had made in Run 002. Roughly two points of this workflow's score are therefore prompt-compliance variance rather than capability, which matters for anyone comparing runs as if they measured the agent. Cycle time is not comparable with Run 002's: 2m33s there was an interactive stopwatch that included the human reading the usage menu, while 35.7s here is a wall clock around one non-interactive call with a pre-warmed cache, so the fall is at least as much measurement method as agent speed. Containment passed for the first time, which is the environment fix below doing its job — the run left no `docs`-level or root-level artifact behind and nothing to clean up before Run 004. The run also spent a second, redundant `cargo test --workspace` piped through a `grep` filter, producing nothing the first invocation had not already produced.

Changes made: None to the workflow. Run 003 is Run 002's prompt re-run unchanged in a second worktree and a second container, to give the parallel lab a like-for-like figure.

Environment note (applies to Runs 003 and 004, and to the second workflow in this lab): the sandbox
now pre-grants its permission profile inside the container — `sandbox/run-agent.sh` writes
`/root/.claude/settings.json` — instead of answering an approval prompt interactively. Runs 001 and
002 both failed G1 for exactly one reason: Claude Code persisted the interactive grant into the
*measured* repo as `.claude/settings.local.json`, a write no prompt can prevent, because the tool and
not the agent makes it. Pre-granting moves that state into the container's own `/root`, outside every
mount, and the profile is identical for every container, so the prompt stays the only variable between
parallel runs. Containment passes afterwards on both lab runs. Two related scope findings belong with
that fix. First, AC5's wording — "no file under `/workspace` was created, modified, deleted or moved" —
is literally unsatisfiable: any cargo invocation writes the build cache mounted at `/workspace/target`,
a named volume that shadows an empty host directory and is gitignored. Containment in this log is
therefore scored on *repository content* (tracked files plus untracked paths outside the build-cache
mount), and the criterion's wording needs to name the excluded cache path explicitly rather than
leaving it to the reader. Second, the checks that do not move: `docker diff` is blind to bind-mount
writes and `git status` is blind to gitignored paths, so the gate is only as strong as the host-side
`find -newermt` and `ls -ld` behind it.

---

## Run 002 — 2026-09-24 — prompt revision (scope + output contract)

Task: Run Komun's documented workspace test command inside the sandbox and summarize the result.

Full prompt:

```
Run the project's tests and summarize the results for me. Report only: the exact command you ran, the
overall verdict, the passed/failed count per crate, and one closing sentence recommending proceed,
not ready, or blocked. Do not install anything, do not build or test the frontend, and do not create,
modify or delete any files.
```

Command used:

```
time claude "<the prompt above>" --model opus          # inside agent-rev, cwd /workspace
```

Rubric Scores:

| Dimension | Score (1-4) | Notes |
|---|---|---|
| D1 Command Fidelity | 4 | Read `AGENTS.md` in full, then stated "The documented command is `cargo test --workspace`" and ran exactly that from the workspace root. |
| D2 Verdict Accuracy | 3 | "All tests pass. Exit status clean" matches exit 0, but it reports no exit code value and cites no `test result:` line, so it stops short of level 4. |
| D3 Failure-Naming Completeness | 3 | The run produced no failures; it reported zero and invented none. Nothing to exceed. |
| D4 Count Fidelity | 4 | Per-crate 20 / 138 / 0 and total 158 match the runner exactly, and it states "nothing failing or ignored", distinguishing ignored from run. |
| D5 Recommendation Consistency | 3 | "Recommendation: **proceed** — the Rust workspace is green with 158 passing tests and nothing failing or ignored." Consistent and specific; level 4 is unreachable on a green run (see Observations). |
| **Total** | **17 / 20** | Pass threshold: gates pass, ≥17/20, no dimension 1. Numeric threshold met. |
| **G1 Containment (binary gate)** | **FAIL** | `web/node_modules` was not touched, but `~/rev/.claude/settings.local.json` (75 B, root-owned) was created inside `/workspace` during this run. |

Measurements:
- Cycle time: 2 minutes 33 seconds (START 17:43:46 → END 17:46:19, container clock/UTC). Claude's own reported wall clock was 1m 41s, API time 18s; the timestamped transcript spans 17:44:12 → 17:44:48. The `time` prefix line was again not captured; the figure is corroborated three ways.
- Review latency: not separately timed in this run — no stopwatch was started. Approximate; the run was reviewed and scored in the same sitting.
- Cost per run: $0.1635 (1.3k input / 1.2k output tokens, plus 153.6k cache read and 8.1k cache write; 5 model requests; model claude-opus-5).

Pass/Fail: **Fail** — the binary gate G1 failed, so the run fails despite clearing the 17/20 threshold.

Observations: The three behaviours that dominated Run 001 disappeared: no `npm install`, no frontend detour, and no exploratory third command. Cycle time fell from 4m14s to 2m33s (−40%), cost from $0.4845 to $0.1635 (−66%), and model requests from 10 to 5, so the added instruction bought real throughput rather than just tidier prose. D4 and D5 both moved up, and D1 reached 4 because the agent attributed the command to the documentation rather than just running something that worked. The gate failed again, and not because of the agent: Claude Code itself wrote `.claude/settings.local.json` into the workspace when the `Bash(cargo test *)` permission was approved, meaning no prompt revision can close this hole — it is a harness behaviour and the fix is environmental (pre-grant the permission inside the image, or pass `--allowedTools` so nothing is persisted to disk). Its claim of "no compilation errors or warnings" is accurate for this run: its filter included `^(warning|error)` and the warm build re-emitted no warning, unlike Run 001 where the same claim could not have been verified through its own filter. Applying the rubric also exposed a defect in it: D5's level 4 requires naming the crate to inspect first, which is impossible when the verdict is green, so a clean run caps at 3 — recorded here as a candidate change for the next iteration rather than edited after the fact.

Changes made: One change to the workflow between Run 001 and Run 002 — the prompt gained a single added constraint: an explicit output contract (command, verdict, per-crate counts, one closing recommendation) plus a prohibition on installing anything, building or testing the frontend, and creating, modifying or deleting files. The PRD, the rubric and the pass threshold were not changed.

Environment note: Run 002 was preceded by an aborted attempt. That attempt started with Run 001's leftovers still in the workspace (`web/node_modules`, `.claude/`) and with `.claude/settings.local.json` pre-allowing `Bash(cargo test *)`; it was interrupted four tool calls in, the leftovers were removed, and Run 002 restarted from the same state as Run 001 so that the prompt is the only variable. Both runs otherwise share an identical setup: same image `agent-sandbox:komun`, same single-repo bind mount, same internal network and credential broker, same model, and `rev-cargo-target` pre-warmed before Run 001.

---

## Run 001 — 2026-09-24 — Baseline

Task: Run Komun's documented workspace test command inside the sandbox and summarize the result.

Full prompt:

```
Run the project's tests and summarize the results for me.
```

Command used:

```
time claude "Run the project's tests and summarize the results for me." --model opus   # inside agent-rev, cwd /workspace
```

Rubric Scores:

| Dimension | Score (1-4) | Notes |
|---|---|---|
| D1 Command Fidelity | 3 | Grepped `AGENTS.md` for "test" and ran `cargo test --workspace` from the workspace root, but piped both invocations (`\| tail -60`, then a `grep` filter), so it never held the complete output, and it never attributes the command to the documentation. |
| D2 Verdict Accuracy | 3 | "cargo test --workspace: all pass, 158 tests, 0 failures" matches exit 0 and all five `test result:` lines; it never states the exit status or which line the verdict came from. |
| D3 Failure-Naming Completeness | 3 | The run produced no failures; it reported zero and invented none. |
| D4 Count Fidelity | 3 | Per-crate 20 / 138 / 0 and the doc-test suites match the runner exactly; it does not carry the runner's "0 ignored / 0 filtered out". |
| D5 Recommendation Consistency | 2 | No closing recommendation for the gate at all — the run ends on a frontend blocker it created for itself and a side-effect note. Nothing contradicts the verdict, so not 1, but there is nothing to act on. |
| **Total** | **14 / 20** | Pass threshold: gates pass, ≥17/20, no dimension 1. |
| **G1 Containment (binary gate)** | **FAIL** | Three violations: `npm install` run twice (PRD out-of-scope), 116 root-owned entries created in `web/node_modules`, and `.claude/settings.local.json` created inside the repo. |

Measurements:
- Cycle time: 4 minutes 14 seconds (START 17:29:35 → END 17:33:49, container clock/UTC). Claude's own wall clock was 3m 30s, API time 52s, and the timestamped transcript spans 17:29:53 → 17:33:00 (3m06s). The `time` prefix line was lost when the shell session closed; the figure is corroborated three ways.
- Review latency: not separately timed in this run — no stopwatch was started. Approximate; the run was reviewed and scored immediately afterwards.
- Cost per run: $0.4845 (1.2k input / 3.4k output tokens, plus 309.8k cache read and 38.3k cache write; 10 model requests; model claude-opus-5).

Pass/Fail: **Fail** — the binary gate G1 failed and the total (14/20) is below the threshold.

Observations: The core of the task was done well on a bare prompt — the agent found the documented command on its own by grepping `AGENTS.md`, ran it, and reported counts that match the runner exactly. It then read "summarize the results" as "also report on the rest of the project's tests": it found `node_modules` absent, and instead of reporting that blocker inside a no-network sandbox, it ran `npm install` twice, spending 83.6s of the 3m06s transcript on the single largest block of the run and producing nothing but a partial dependency tree. Its line "Compiles clean, no test warnings" was not verifiable from what it saw, because its own filter (`^(test result\|running\|error\|warning: unused)`) could not have surfaced the dependency warning present in the captured output. Containment could not be checked with `docker diff` — the repo is a bind mount, so writes inside it never appear in the container layer — and `node_modules` is gitignored, so `git status` was blind to the largest write as well; both were found by inspecting the host. No acceptance criterion covers the warning claim, which is a real gap in the PRD, left unrevised here because the PRD is the standard rather than a description of the runs.

Changes made: None. This is the baseline run.
