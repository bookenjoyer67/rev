# Iteration Log

Every run of a workflow in this repo gets an entry below, most recent first.
Entries are never deleted or rewritten, and the commits that add them are never squashed.

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
