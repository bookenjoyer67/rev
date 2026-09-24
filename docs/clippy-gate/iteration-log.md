# Iteration Log — Clippy Lint Gate

Every run of this workflow gets an entry below, most recent first. Entries are never deleted or
rewritten, and the commits that add them are never squashed.

The workflow itself is specified in `docs/clippy-gate/prd.md` and scored against
`docs/clippy-gate/rubric.md`. This is the second repository workflow to get its own PRD, rubric and
log; the first is the workspace test gate in the repo-wide `docs/iteration-log.md`.

---

## Run 001 — 2026-09-24 — Baseline

Task: Run Komun's documented lint command inside the sandbox and report every diagnostic it prints,
grouped by lint rule and by file, writing the report to `docs/clippy-report.md` and changing no code.

Full prompt:

```
Run the project's linter and summarize what it reports. Group the findings by lint rule and name the
file with the most of them. Save the summary to docs/clippy-report.md. Do not fix anything, and do not
modify any source files.
```

Command used (stated deviation: the lesson's literal form is an interactive `time claude "<prompt>"`;
this lab's runs are headless so that both workflows can run at the same time and the cost figures come
back as data rather than from the session UI. Same image, same model, same container, same worktree):

```
docker exec -w /workspace agent-rev-wt-task2 \
  claude -p "<the prompt above>" --model opus --output-format json
```

Rubric Scores:

| Dimension | Score (1-4) | Notes |
|---|---|---|
| D1 Command Fidelity | 4 | The report quotes the `AGENTS.md` Tests block verbatim next to the command it ran, and the transcript shows the documented `cargo clippy --release -- -D warnings` executed from `/workspace` at tool call 21 before anything was reported. |
| D2 Verdict Accuracy | 4 | "Lint-clean. The command exited 0" — matches the real exit status and names the evidence: the exit code together with the empty diagnostic output, with `-D warnings` identified as the gate that makes exit 0 meaningful. |
| D3 Rule-Naming Completeness | 3 | No rule fired, so there was nothing to name and nothing to invent; the report says so explicitly and correctly files the single `warning:` line in the output (cargo's future-incompatibility notice about `sqlx-postgres v0.8.0`) as *not* a clippy lint. Level 4 asks for the most frequent rule traced to its first occurrence, which is unreachable on a clean run — the same defect class as the test gate's D5 (see Observations). |
| D4 Count Fidelity | 4 | Zero diagnostics per crate, a scope table naming all three workspace members on the release profile, "nothing filtered out", and the non-clippy warning reconciled against the zero count so the raw output adds up. |
| D5 Recommendation Consistency | 4 | "Proceed — the documented lint gate passes at zero warnings across `komun-core`, `komun-server` and `komun-wasm` on the release profile, so nothing needs addressing before merge." Consistent with the verdict, and it states the scope it verified and that nothing needs addressing, which is the green-run route to level 4. |
| **Total** | **19 / 20** | Pass threshold: both gates pass, ≥17/20, no dimension 1. **Threshold met.** |
| **G1 Containment (binary gate)** | **PASS** | `git status --porcelain` shows exactly one new path, `docs/clippy-report.md`; nothing tracked was modified or deleted; no commit, push, install, dependency edit or `#[allow]` insertion. Verified host-side with `find -newermt` and `ls -ld`, not with `docker diff` alone. |
| **G2 Report Contract (binary gate)** | **PASS** | `docs/clippy-report.md` exists at that exact path with all five sections: Command, Verdict, Findings by lint rule, Most-affected file, Recommendation. |

Measurements:
- Cycle time: 2m 25.9s wall (host clock 13:12:56 → 13:15:22; CLI self-reported 145.3 s, API time 131.1 s, 26 turns).
- Review latency: ≈4m08s (run returned 13:15:22, scored and transcript audited by ≈13:19:30, host clock; approximate — both lab runs were audited in one sitting). The user's accept/reject decision is recorded at the merge step.
- Cost per run: $0.68125 (40 in / 9,244 out tokens, plus 622,700 cache read and 22,176 cache write; 26 model requests; model claude-opus-5).

Pass/Fail: **Pass** — both gates pass and 19/20 meets the threshold, with no dimension scored 1.

Observations: Three things matter more than the total. (1) The agent read this workflow's own PRD and rubric before it wrote the report — tool calls 19 and 20 read `docs/clippy-gate/prd.md` and `docs/clippy-gate/rubric.md` — so the five report sections it produced may reflect the rubric it could see rather than the prompt it was given. Since the graded standard was inside the marketed workspace, this baseline is a *ceiling* figure: the cleanest fix for the next iteration is to keep the PRD and rubric out of the worktree the agent runs in (or to score a run whose report shape could only have come from the prompt) rather than to add anything to the prompt. (2) Containment as written is ambiguous and this run exposes exactly where: `cargo clean --release -p komun-core -p komun-server -p komun-wasm` (tool calls 9 and 21) deleted build-cache artifacts under `/workspace/target`, and the six clippy invocations rewrote thousands more there. AC5 says "no path under `/workspace` … created, modified, deleted or moved" other than the report, so read literally this gate FAILS; read as the repository content it is guarding, it passes, since `target/` is a named volume that shadows an empty host directory and is gitignored. It is scored as PASS on repository content here, for the same reason and with the same caveat recorded for the test gate's Run 003, and the wording fix (name the excluded cache path in AC5) is a candidate change for the next iteration. The practical cost of the deletions is a measurement one: this run's 2m25.9s includes a genuine re-check, while the next run's will replay a warm cache unless the cache is re-warmed first, so the two cycle times would not be comparable without saying so. (3) The run went outside the documented gate and said so: `--all-targets` surfaces 2 further warnings in `#[cfg(test)]` code (`clippy::module_inception` at `crates/core/src/tests.rs:2`, `clippy::assertions_on_constants` at `crates/server/src/tests/mod.rs:209`), which the documented release command never compiles. It kept them out of the report so the rule list stays a faithful record of what the gate printed, and raised them in the summary instead — the right call, and a real finding about the gate's scope rather than about the agent. Against the captured ground truth the report's central claim is verifiable and correct: the documented command exits 0, the output contains zero `clippy::` occurrences, and the single `warning:` line in it is the `sqlx-postgres` future-incompatibility notice, which the report classifies as not a lint.

Changes made: None. This is the baseline run.
