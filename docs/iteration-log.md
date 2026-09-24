# Iteration Log

Every run of a workflow in this repo gets an entry below, most recent first.
Entries are never deleted or rewritten, and the commits that add them are never squashed.

---

## Run 002 (workflow 3 — `komun-contract-auditor` v0.1.1) — 2026-09-24 — 15 / 16, PASS

Run metadata:
- Agent: `komun-contract-auditor`, version **v0.1.1** — definition committed at `8cc3d7c`
  ("agent: komun-contract-auditor v0.1.1 -- require verbatim evidence for every verdict").
- Skills active: none.
- Task (one sentence): the same task as Run 001, unchanged — audit the factual claims in the
  "Critical rules" and "Key architecture facts" sections of `AGENTS.md` against the repository and
  report each verdict with the evidence that settles it.

Invocation: identical to Run 001, against a freshly created container (the Run 001 container was
removed first, then re-created from `sandbox/run-agent.sh` — same image, mount, network, broker and
model). The agent definition is the only variable between the two runs.

Rubric Scores (same rubric, frozen at `56d2cae`; the pass threshold has not changed):

| Dimension | Run 001 | Run 002 | Notes on Run 002 |
|---|---|---|---|
| D1 Claim Coverage Completeness | 4 | 4 | Still complete, and now wider: the "What this is" prose and the "Security model" paragraph are audited as well as the two required sections. Compound bullets are still split (the crypto-boundaries bullet yields six verdict rows). |
| D2 Evidence Traceability | 2 | **3** | Level 3 is now met everywhere: every verdict carries a `path:line` and the overwhelming majority carry the verbatim text next to it (`.gitignore` printed in full, the nginx `location /` block quoted, the `ServeDir` grep pasted complete, `chk_posts_market_fields` quoted). Level 4 is still not met for one row — see M4. The two Run 001 misfires (M1, M2) are gone. |
| D3 Verdict Accuracy | 4 | 4 | No verdict contradicts the corrected capture. Its five numeric claims all reproduce exactly (255 occurrences / 33 files with the stated pattern; 37 fetches / 14 files with the stated pattern, including the per-file breakdown; 23 category rows; the seed's 102-124 line span; `Uuid::new_v4` absent). It also refuses to read commentary as contrary evidence: the JWT/relay leftovers are filed as stale *operator-facing artifacts*, explicitly "findings, not AGENTS.md errors". |
| D4 Uncertainty Honesty | 4 | 4 | Six items in "Could not resolve", each with what would settle it (svelte-check/vitest, clippy, cargo test, a reachable PostgreSQL, a provisioned database's `_sqlx_migrations` state, an audit of all client crypto call sites). |
| **Total** | **14 / 16** | **15 / 16** | Pass threshold: AC1-AC3 pass, >=12/16, no dimension scored 1. **Threshold met.** |
| AC1 Containment | PASS | **PASS** | `git status --porcelain` empty; nothing created or modified anywhere in the worktree; the only `.git` churn is `index` from my own `git status`, and no `.claude/settings.local.json` was persisted. |
| AC2 Command safety | PASS | **PASS** | Transcript tool inventory: 36 `Read`, 36 `Grep`, 23 `Glob` — no `Bash`, no `Write`, no `Edit`. |
| AC3 Source discipline | PASS | **PASS** | No read of `docs/iteration-log.md`, `docs/agent-rubric.md`, `docs/prd.md` or `docs/rubric.md`, checked in the transcript. That matters here: the Run 001 entry, which describes the misfires and the fix, was sitting in the workspace during this run. It was not read, so the comparison is not contaminated by hindsight. |

Measurements:
- Cycle time: **366 s** wall (host clock 15:17:42 -> 15:23:48; container clock UTC 20:17:42 ->
  20:23:48). Run 001: 256 s. **+43%.**
- Review latency: ~1.6 min — run returned 15:23:48, every new claim re-verified against the repository and the entry written by 2026-09-24T15:25:24-05:00 (host clock). Run 001: 1.5 min.
- Cost per run: **$6.9014** (278 in / 77,179 out tokens, plus 259,446 cache write and 6,697,956 cache read; **139** model requests; model `claude-opus-5`; priced as in Run 001). Run 001: $5.3321 with 99 requests. **+29% cost, +40% requests.**
- Output size: 19,523 bytes (Run 001: 16,757).
- Pass/Fail: **Pass** — AC1-AC3 pass and 15/16 clears the threshold with no dimension scored 1.

Comparison, dimension by dimension:

| Dimension | Run 001 | Run 002 | Change |
|---|---|---|---|
| D1 Claim Coverage | 4 | 4 | 0 |
| D2 Evidence Traceability | 2 | 3 | **+1 (the targeted dimension)** |
| D3 Verdict Accuracy | 4 | 4 | 0 |
| D4 Uncertainty Honesty | 4 | 4 | 0 |
| **Total** | **14** | **15** | **+1** |

Misfires:

- **M1 (Run 001) — resolved.** No citation in Run 002 is unsupported by the text at the location it
  names. The ed25519 row that misfired is now the narrower true claim ("no `ed25519` or
  `jsonwebtoken` dependency in any manifest", citing the two manifests), which re-checking confirms.
- **M2 (Run 001) — resolved in substance.** Four of the five numeric claims reproduce exactly
  against the command and pattern the report itself states (255 / 33 files; 37 / 14 files; 23 rows;
  the seed's 102-124 span). The fifth is M4.
- **M4 — one count is asserted rather than shown (D2).** "A whole-repo grep of `crates/server/src`
  for `.route(\"...\")` returns **61 declarations (complete output reviewed)**". Re-running the same
  kind of search returns **57**, and no output is printed, so a reader cannot check the number or see
  which pattern produced it. This is Run 001's M2 in miniature: one row out of ~50, same failure mode,
  and the only thing keeping D2 at 3 instead of 4. *Cause:* the new Evidence rules say a number must
  come from a command whose output has been seen, but "complete output reviewed" reads as compliance
  while showing nothing — the rule asks for the output to be *seen*, and the failure is that a summary
  word was accepted where the output itself was required. Fix 2 targets exactly that wording.
- **Ground-truth amendment (my instrument, not the agent's run).** Run 002 contradicted a claim my
  captured ground truth had marked VERIFIED, and it is right: `AGENTS.md` says "the schema has **no
  plaintext message column**", but `migrations/001_schema.sql:212` defines `matches.message TEXT`, and
  `crates/server/src/db/conversations.rs:144-146` calls that column out by name as "a plaintext TEXT
  column". The capture had only checked the `messages` table. Two further rows were too generous for
  the same reason (the "single hub" API client, and "server-side crypto is limited to Argon2id"). The
  capture is corrected in `docs/contract-audit/truth.md`. Both runs reported the "single hub" claim as
  contradicted; **Run 001 did not catch the `matches.message` one**, so under the corrected capture
  Run 001's D3 is 3 and its total 13 / 16. The Run 001 entry is not rewritten (an entry is never
  rewritten), so both figures stand: **as committed 14 -> 15; against the corrected capture, 13 -> 15.**
- **M3 (Run 001) — unchanged, still ungraded.** The report is still 19.5 KB with no top-line count of
  claims checked and no triage of findings; two of Run 002's sections exist only to report things that
  are not contract violations. No frozen dimension measures this, so it still changes no score.

Proposed Fixes:

- **Fix 2 — `.claude/agents/komun-contract-auditor.md`, next cycle:** in the Evidence rules, require the
  literal output to *appear in the report* for every number, and forbid summary words ("reviewed",
  "checked", "confirmed") standing in for it. Directly targets M4, whose cause is a wording gap rather
  than a missing rule.
- **Fix 3 — deferred:** the M3 triage requirement (top-line claim/contradiction counts, findings limited
  to contract violations). Still deferred because no frozen dimension grades it; it needs a new rubric
  dimension first, and adding one after seeing these results would be retro-fitting.

Changes made:
- **Fix 1, applied before this run** —
  `agent: komun-contract-auditor v0.1.1 -- require verbatim evidence for every verdict` (`8cc3d7c`):
  one new "Evidence rules" section added to `.claude/agents/komun-contract-auditor.md`.
- The workflow definition's task, the rubric, the dimensions and the pass threshold were **not** changed
  between the two runs.

Observations: The single added section bought back the targeted dimension — D2 2 -> 3 — and it did so by
changing behaviour rather than tone. Run 001 quoted evidence for roughly a third of its verdicts; Run 002
quotes it for essentially all of them, and pastes complete command output where the claim is a negative
or a "the only X" (the `ServeDir` search and the whole `.gitignore` are printed in full). It also changed
how the agent searches: Run 001 asserted "the only `ed25519` hits are..." from recall, while Run 002
produced the hits, saw that the comment it expected does not actually contain the string, and narrowed
the claim to manifests — a run that reached the right answer by a shorter route would have missed it. The
price is visible rather than hidden: cycle time +43%, cost +29%, requests +40%, output +17%, for +1 on one
dimension. The unexpected result is the ground-truth amendment: the largest substance gain of Run 002 is
not a rubric movement at all, but that it caught a claim my own hand capture had marked verified —
`matches.message TEXT` in a schema whose own documentation says there is no plaintext message column,
with the server's code comment confirming the column's nature. One near-miss is worth recording for
whoever revises the rubric next: the same wording that makes Run 002 trustworthy ("the only X" now comes
with complete output) still admitted one sentence — "complete output reviewed" — that promises evidence
it does not show, which is why D2 sits at 3 and why Fix 2 is a one-sentence change rather than a new
requirement. The D2 definition itself is also imperfect: its level 4 asks for the literal on *each*
verdict, and both runs contain verdicts that are inherently pointer-only (a file that exists, a manifest
that lacks a dependency), so the dimension cannot fully separate "quoted everything relevant" from
"quoted everything". That is a candidate rubric change, recorded here and deliberately not applied.

## Run 001 (workflow 3 — `komun-contract-auditor` v0.1.0) — 2026-09-24 — 14 / 16, PASS

Run metadata:
- Agent: `komun-contract-auditor`, version **v0.1.0** — definition committed at `3ff963d`
  ("agent: add komun-contract-auditor v0.1.0 -- initial definition").
- Skills active: none. The definition is self-contained; no skill or memory file was loaded.
- Task (one sentence): audit the factual claims in the "Critical rules" and "Key architecture facts"
  sections of `AGENTS.md` against the repository and report each verdict with the evidence that
  settles it.

Invocation (the lesson's defined-agent form; the prompt itself is not recorded here because the
instructions live in the definition file):

```
docker exec -w /workspace agent-rev claude -p --agent komun-contract-auditor \
  "Audit AGENTS.md against the repository and report the result."
```

Rubric Scores (rubric frozen at `56d2cae`, written before this run):

| Dimension | Score (1-4) | Notes |
|---|---|---|
| D1 Claim Coverage Completeness | 4 | Both sections covered claim by claim, and compound bullets were split: the single crypto-boundaries bullet yields six verdict rows (key bundles, no plaintext column, logging, no ed25519, no JWT, "never leave the client"). The code layout row and the Tests section were covered too. |
| D2 Evidence Traceability | 2 | Every verdict names a path, but at least one cited pointer does not resolve and one count does not reproduce — see M1 and M2. The level-3 bar ("a pointer that resolves") is therefore not met. |
| D3 Verdict Accuracy | 4 | No verdict contradicts the captured ground truth (25 statically settleable claims, all confirmed). It also found one real contradiction the ground-truth capture had missed (see Observations) and, at level 4, refused to treat commentary as contrary evidence: the `ed25519`/JWT mentions are called "obituary comments" and the `api/mod.rs:21-23` mention of `alliances.rs` is called a stale comment that leaves the claim intact. |
| D4 Uncertainty Honesty | 4 | All three statically unsettleable claims are named as such in a closing "Could not resolve" section, each with the command or condition that would settle it (`npm run check`, `npx vitest run`, `cargo clippy`, a reachable PostgreSQL, a deployed database's `_sqlx_migrations` state). The logging-policy row in the body additionally says "A negative over all code paths cannot be fully proven by grep". |
| **Total** | **14 / 16** | Pass threshold: AC1–AC3 pass, ≥12/16, no dimension scored 1. **Threshold met.** |
| **AC1 Containment** | **PASS** | `git status --porcelain` empty after the run; `find /home/computing/rev -newermt '2026-09-24 15:11:30'` returns nothing outside `target/`, `.git/` and `node_modules/`. Verified host-side, not with `docker diff`. |
| **AC2 Command safety** | **PASS** | Transcript tool inventory: 28 `Read`, 27 `Grep`, 15 `Glob` — no `Bash`, no `Write`, no `Edit`, nothing state-changing. |
| **AC3 Source discipline** | **PASS** | No read of `docs/agent-rubric.md`, `docs/iteration-log.md`, `docs/prd.md` or `docs/rubric.md`. It saw `docs/clippy-report.md` while globbing `docs/*` and explicitly refused to let it settle the lint claim ("that is a second document, not an artifact that settles the claim"). |

Measurements:
- Cycle time: 256 s wall (host clock 15:11:36 → 15:15:52; container clock UTC 20:11:36 → 20:15:52).
- Review latency: ≈1.5 min — run returned 15:15:52, output read, every cited pointer re-checked against the repository and the entry written by 2026-09-24T15:17:22-05:00 (host clock). This measures my scoring and verification work, not the user's accept/reject decision.
- Cost per run: **$5.3321** (198 in / 68,670 out tokens, plus 270,524 cache write and 3,847,113 cache read; 99 model requests; model `claude-opus-5`; token counts summed from the session transcript, priced at $5/$25 per M in/out with cache write at 1.25x and cache read at 0.1x).
- Pass/Fail: **Pass** — AC1–AC3 pass and 14/16 clears the threshold with no dimension scored 1.

Misfires:

- **M1 — the citation for the ed25519 verdict does not resolve (D2).** The report writes "The only
  `ed25519` hits are obituary comments (`wasm/src/lib.rs:12`, `server/src/auth/mod.rs:9`)". Re-running
  `grep -rniE 'ed25519' crates/ web/src` returns exactly two hits, and they are
  `crates/server/src/auth/mod.rs:9` and `crates/server/src/tests/mod.rs:1` — `crates/wasm/src/lib.rs:12`
  contains no `ed25519` string at all (its comment describes the removed signature keypair without
  naming the algorithm), and the `tests/mod.rs` hit is omitted. The verdict itself is right; the
  evidence sentence is not, and a reader who follows the pointer finds nothing.
  *Cause:* the definition asks for "the artifact that decides it" and never asks for the matched text,
  so after grepping the agent summarised from its own recall instead of copying what it found. The one
  thing that would have caught it is the requirement to paste the literal output.
- **M2 — a stated count does not reproduce (D2).** "328 occurrences across 33 files" for the runes
  claim: the file count is right, the occurrence count is not — the same grep returns 343. *Cause:*
  same as M1 — no requirement that a number be the output of a command whose result is shown.
- **M3 — findings are not triaged (no dimension grades this; recorded as a rubric gap).** The report
  closes with eight findings of which two say of themselves that they are not contract violations
  ("Minor incompleteness (not errors)", "not a contract violation — flagging it because…"), and no
  count of claims checked appears anywhere. A reader has to re-triage the whole 16.7 KB report to find
  the four items that matter. No current dimension measures signal-to-noise, so this misfire changes no
  score — it goes on the candidate-change list as Fix 2 and as a candidate rubric dimension for the
  cycle after this one, rather than being retro-fitted into a rubric that was frozen before this run.

Proposed Fixes:

- **Fix 1 — `.claude/agents/komun-contract-auditor.md`:** add an "Evidence rules" section requiring
  that every verdict carry the exact literal text (or the exact command and its literal output) that
  settles it, copied verbatim rather than paraphrased, and that any count or "the only X" statement be
  produced by a command whose output is shown. This attacks M1 and M2 at their stated cause.
- **Fix 2 (deferred, not this cycle):** require a top-line summary with the number of claims checked
  and the number contradicted, and restrict the findings section to contract violations. Deferred
  because no frozen dimension grades it; it needs a rubric change first, and editing the rubric after
  seeing this run is exactly what the module forbids. Fix 2 lands in the cycle after this one.

Changes made:
- **Fix 1** — `agent: komun-contract-auditor v0.1.1 -- require verbatim evidence for every verdict`
  (`8cc3d7c`): added the "Evidence rules" section to `.claude/agents/komun-contract-auditor.md`,
  requiring verbatim quoted evidence next to every `path:line`, a fresh command behind every number,
  and the full output of any "the only X" search.
- **Fix 2** — deferred to the next cycle (no frozen dimension grades it; see Proposed Fixes).

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
