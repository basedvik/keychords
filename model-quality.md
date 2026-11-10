# Model quality: chiz vs the Mac mini

How each machine's model performs on keychords tasks, judged against the implementation Claude (Opus 5.5) would write. Updated as each task finishes.

| Machine | Model | Serving |
|---|---|---|
| chiz | qwen3.8-27b, dense, Q8_0, 262k context | llama.cpp on 3× RTX A4000, MTP speculative decoding, 1 slot, ~71 tok/s single-stream |
| Mac mini | qwen3.6-35b-a3b-uncensored, MoE with 3B active, Q4_K_M, 131k context | llama.cpp on Apple silicon (32 GB), 4 slots |

Both run headless through `opencode run`, each in its own git worktree, with a task spec that names the files it may edit and the verification command.

## Rubric

Each task is scored 1–5 on:

| Axis | What it measures |
|---|---|
| **Correctness** | Do the acceptance tests pass, and does the code also handle edge cases the tests don't pin down? |
| **Spec fidelity** | Did it implement what the doc comments say, or just patch until the tests passed (hard-coding, special cases)? |
| **Rust quality** | Idiomatic code, no needless allocation or cloning, clear naming, clippy clean, matches the surrounding style. |
| **Discipline** | Stayed inside its allowed files, didn't edit tests, didn't add dependencies, reported honestly. |
| **Efficiency** | Wall-clock time, retries needed, and whether it looped or wandered. |

**Reference:** what Claude would have written for the same spec. Differences are called out concretely.

## Results

| Task | Machine | Difficulty | Result | Time | Retries | Correct | Spec | Rust | Discipline | Efficiency |
|---|---|---|---|---|---|---|---|---|---|---|
| T1 chords | mini | mechanical | **merged** | 8.4 min | 0 | 4 | 5 | 3 | 5 | 3 |
| T2 keymap | mini | mechanical, some logic | **merged** (review fixed a latent bug) | 11.2 min | 0 | 3 | 4 | 3 | 5 | 3 |
| T5a SMF | mini → chiz (thinking off) | byte-level, spec-heavy | mini #1 failed (stale worktree, 8k cap); chiz running | 5.3 min (#1) | 2 | | | | | |
| T4 glide | chiz (thinking **off**) | stateful scheduling | **merged** after 1 review-note retry + review fix | 3.5 + 2.2 min | 1 | 4 | 4 | 4 | 5 | 4 |
| T8 sequencer | mini | mechanical + edge cases | **merged** | 6.2 min | 0 | 5 | 5 | 4 | 5 | 5 |
| T3 voicing | chiz | algorithmic | **merged** (attempt 2); differential-tested vs reference | 10 + 13 min | 1 | 5 | 5 | 4 | 5 | 1 |

## Per-task notes

### T1 chords: Mac mini, merged

- **Outcome:** 4/4 tests and clippy clean on its own; one file touched; no test edits. Final report was accurate and concise.
- **Timeline:** dispatched 14:01:28, finished 14:09:51 (503 s). About 5 min of that was reading and reasoning while it shared the server with two other workers (see the serving note below). Once alone, it went from first edit to green in about 2 min.
- **Unicode corruption (notable).** Its first attempt wrote `♭` as `₵` and `♯` as `♫` (`"A7₵9"`, `"C♫"`), even though it was copying from a test file that showed the right characters. It recovered by itself, switching to `\u{266D}`/`\u{266F}` escapes after the first failing run. This looks like a tokenizer or Q4 quantization weakness on uncommon symbols. **Implication:** give this model `\u{…}` escapes in specs for any non-ASCII output.
- **Versus the reference:**
  - `index` and `from_index` are two hand-written 24-arm `match` tables. I'd have written `self as usize` and `Self::ALL.get(i).copied()`: one line each, and they can't drift out of sync with `ALL`. Correct, but more code to maintain.
  - `root_name(pc)` indexes `[pc as usize]` without `% 12`, so it panics on `pc ≥ 12` even though it's public. `Chord::new` wraps the root but `root_name` doesn't. It's an edge case the tests didn't cover, and the reference guards it.
  - `pitch_classes` and `name` are the same as what I'd write.
- **Serving note:** the first ~6 min ran with 3 workers sharing the mini's 4 slots, at about 7.7 tok/s per stream (31 tok/s total). Alone on one slot it measured **39.5 tok/s**. The mini now runs one task at a time.

### T2 keymap: Mac mini, merged after a review fix

- **Outcome:** 10/10 tests and clippy clean on its own; one file touched; no test edits; accurate report (it also flagged the `chords` stub warnings as a spec issue).
- **Timeline:** 14:09:53 → 14:21:02 (11.2 min). The first compile had 16 errors: variants used without the `KeyAction::` prefix (it copied the test file's imports into its head) and an invented method, `char::is_ascii_letter`. It fixed them in about 3 iterations, and once failed an edit with "oldString not found" before retrying.
- **Latent bug found in review:** `SplitDecoder` computed the root as `(note − split).rem_euclid(12)`, relative to the split, so it's wrong for any split that isn't a C. The tests used 48 and 60 and missed it. Claude added a test (F split), confirmed it failed, and fixed the line to `note % 12`. The **test gap was Claude's**; the **bug was the model's**: it reasoned about notes relative to the split instead of reading "its pitch class" in the doc comment.
- **Versus the reference:**
  - It uses `Quality::ALL[22]`-style magic indices throughout. This was **spec-induced:** the spec said to use `ALL[i]` so it wouldn't call stub functions, and the model generalised that to the whole key table. Claude would write `Quality::Dom7s9` and so on. An index slip produced a duplicate `ALL[19]` mid-run, which it caught itself.
  - The dead-key handling is a reasonable reading of the spec.
  - The `else { // ignored }` empty branch would be dropped in the reference.

### T3 voicing: chiz, attempt 1 failed with no output

- **What happened:** it read the spec, the tests and the `chords` crate (14:01–14:02), then opened one reasoning turn and **never wrote a line**. chiz's slot showed `decoded 32000, remain 0` against a 55.6k-token prompt. The whole reply budget went on thinking. opencode got back an empty reply, treated the session as finished, and exited cleanly at 600 s with no error.
- **Whose fault:** partly the harness. opencode caps each reply at 32,000 tokens (`OPENCODE_EXPERIMENTAL_OUTPUT_TOKEN_MAX`), below the 32,768 set in your config. That's tight for a reasoning model given an algorithm-design task. Partly the model: it tried to solve the entire crate in its head instead of writing code and letting the tests guide it.
- **Context was not the issue:** 55.6k of 262k used.
- **Mitigations for attempt 2:**
  - output cap raised to 100k (project `opencode.json` plus the env var)
  - the spec now requires incremental steps
  - `dispatch.sh` flags runs that end with no changes
- **Lesson:** a dense 27B reasoning model will happily think for 30k+ tokens on a hard spec. Give it room *and* tell it to iterate.


### T3 voicing: chiz, attempt 2 merged

- **Timeline:** dispatched 14:12:45. It reasoned for ~9 min (≈25k tokens at ~61 tok/s), first edit 14:22:46, 3/9 tests at 14:23:45, 7/9 at 14:24:30, **9/9 and clippy clean at 14:25:21**, report at 14:25:41 (776 s total). Once it started writing, it went from first edit to green in **2.5 min**: compile errors (a missing lifetime, type mismatches) fixed in about 4 iterations.
- **It flagged two genuine ambiguities in Claude's spec:** the tie-break for a one-chord loop, and what the "largest single-voice move" means when voice counts differ. That's high-value reporting.
- **Differential test against Claude's reference** (2,000 random progressions, octaves −1..1, 1–8 chords):
  - `voice_cycle` total cost was **identical in all 2,000**. Both are exact optimisers.
  - The greedy `next` differed in 191 progressions. Isolated per step, **every difference is a tie on motion between voicings with different voice counts (40 cases), and 0 are real errors.** chiz breaks those ties with the largest move along the DTW alignment; Claude's reference ignores that tie-break when sizes differ. **chiz's choice is the more principled one.**
- **Versus the reference:**
  - Structure is comparable: a DTW that returns the alignment path (the reference returns only the cost), plus clean `pick_least_motion`, `pick_centre` and `score` helpers.
  - Slightly more verbose: hand-written `if` chains for the tie-break tuple, where the reference uses `min_by_key` on a tuple.
  - Edge case: `pick_*` and `voice_cycle` index `cands[0]` and would panic if a chord had no candidates (impossible with the current bands, but unguarded). The reference returns empty.
  - `candidates` builds the cartesian product by cloning vectors; fine at ≤3^5.
- **Verdict:** **correct and production-quality code**, but very expensive to get: 2 attempts, ~21 min of reasoning, ~64k output tokens (half wasted) for ~200 lines that took the reference ~2k tokens to write.

### T5a SMF: Mac mini, attempt 1 failed with no output

- **What happened:** read both files, then its first reasoning reply **hit `finish: length` at exactly 8,192 tokens**, then a second reasoning turn ended with nothing (317 s total). The new `[NO CHANGES]` check caught it.
- **Root cause (harness, not model):** its worktree was created at 14:01 for the run that was killed, and `dispatch.sh` reused it without merging `main`. So it never had the project `opencode.json` (committed 14:12) that raises the mini's output cap to 32,768. Verified with `opencode debug config`: the stale tree resolved 8,192; after a fast-forward it resolves 32,768.
- **Fix:** `dispatch.sh` now merges `main` into any reused clean worktree. T5a is re-queued behind the mini's current tasks.
- **Model note:** at 8k the mini also spends its whole reply budget reasoning on a spec-heavy task. With the cap at 32k it may still reason long; worth retrying with thinking off if the retry repeats this.

## Chronological timeline

All times are local (Nov 10, 2026), taken from logs, git and opencode's session database.

| Time | Event | Thinking / decision |
|---|---|---|
| ~13:50 | Plan written (`deploy-plan.md`). Contract-first: Claude writes interfaces and failing tests; workers fill in bodies. | Weak models need a compiler-and-tests oracle and a narrow file scope. |
| 13:55 | You choose a standalone-only Rune app. | Rune owns its window through winit, so it can't run inside a DAW's plugin window. |
| 13:57–14:00 | Phase-1 skeleton: `chords`, `keymap`, `voicing`, `midi::smf`, with 30 failing tests. Voicing tests encode the screenshot ground truth (hand-verified, including tie-break rules). | — |
| 14:00 | The safety check blocks a commit: chiz's API key was hard-coded in `dispatch.sh`. Rewritten to read it from your opencode config at runtime. | Correct block. |
| 14:01:28 | Dispatched **T3 to chiz; T1, T2 and T5a to the mini** (3 at once). | *Mistake:* treated the mini's 4 slots as 4× capacity. |
| 14:07 | You ask about the mini's single slot. Measured **7.7 tok/s per slot with 4 busy (31 total)**. Killed T2 and T5a; the mini now runs one task at a time. | Shared KV cache and GPU mean parallel slots split throughput; they don't multiply it. |
| 14:07 | *Mistake:* reported "14:12" and "17 min in" without checking the clock; it was 14:07. Corrected, and switched to clock-checked updates plus an event monitor. | — |
| 14:07:34 | T1 first edit. Mini alone measured **39.5 tok/s**. | 5× faster per stream once alone. |
| 14:07:58 | T1 first test run: 1/4. The model corrupted `♭` to `₵` and `♯` to `♫`. | Tokenizer/Q4 weakness on rare symbols. |
| 14:09:19 | Rune shell builds (44 s cold) and renders headless (`--snap`). | UI framework risk gone. |
| 14:09:51 | **T1 done** (503 s, 4/4, self-fixed Unicode with `\u{}` escapes). Merged. | — |
| 14:11:28 | **T3 attempt 1 fails silently:** 600 s, 0 lines. chiz slot: `decoded 32000, remain 0`. | One reasoning turn used the whole opencode reply cap (32,000, hard-coded unless `OPENCODE_EXPERIMENTAL_OUTPUT_TOKEN_MAX` is set). Context was fine: 55.6k of 262k. |
| 14:12 | Fixes: project `opencode.json` raises the output caps (chiz 100k, mini 32k); `dispatch.sh` flags no-change runs; T3 spec says work incrementally. | *Mistake:* edited `dispatch.sh` while T2's copy was running (see 14:21). |
| 14:12:45 | T3 attempt 2 dispatched. | — |
| 14:13–14:20 | Claude writes the app engine (cpal, glider, synth, MIDI), controller and Rune view against the stub interfaces. App compiles. | Rune is pre-release and git-only, so the local models have never seen its API; Claude does the UI. |
| 14:15:42 | T2's first compile: 16 errors (missing `KeyAction::` prefixes; invented `char::is_ascii_letter`). | — |
| 14:16:58 | T2: 10/10 tests. Clippy tripped on the `chords` *stubs* in its stale worktree; Claude merged `main` into it so the model wouldn't edit out of scope. | Long-lived worktrees go stale; sync them when dependencies merge. |
| 14:19 | T3 attempt 2 has reasoned for **20.8k tokens at 60.9 tok/s** and written nothing, despite the incremental instruction. Deadline set for 14:25. Claude starts a reference `voicing` as fallback and yardstick. | — |
| 14:21:02 | **T2 done** (11.2 min). Its `dispatch.sh` process crashed on `line 32: syntax error` because the file had been edited under it, so no "finished" line and the watcher never fired. The chain still started T5a. | Bash reads scripts lazily; never edit a running script. Queues now launch from a frozen copy. |
| 14:21:37 | Claude's reference `voicing`: 9/9 and clippy clean (one bug of its own fixed: empty loop). Kept on `ref/voicing`. | The spec is solvable as written. |
| 14:21:59 | T3 attempt 2 leaves reasoning after about 25k tokens (~9 min) and acts. | — |
| 14:22:46 | **chiz's first line of code of the session**, 21 min after first dispatch. | — |
| 14:23:45 | T3: 3/9 tests (`bass_note`, `motion`, `candidates` work). | — |
| 14:24 | **Review catches a T2 bug the tests missed:** the split decoder computed the root relative to the split, which is only correct for splits that are multiples of 12. Claude adds a test with an F split (fails, as expected), fixes one line, merges. | The test gap was Claude's; the bug was the model's. |
| 14:25:21 | **T3 attempt 2: 9/9 and clippy clean.** First edit to green in 2.5 min after ~9 min of reasoning. | — |
| 14:26 | Differential test of T3 vs Claude's reference on 2,000 random progressions: identical loop costs; 40 greedy differences, all motion ties with different voice counts (the ambiguity chiz flagged), 0 errors. **T3 merged.** | chiz's tie-break is better than the reference's. |
| 14:26:19 | **T5a fails with no output:** `finish: length` at 8,192. Root cause: stale worktree from the killed 14:01 run, so no project `opencode.json`. | Same stale-worktree problem as T2. `dispatch.sh` now syncs reused worktrees. |
| 14:27 | Mini queue continues (T8 → T6 → T5b, then the T5a retry). | — |
| 14:28 | **Experiment:** confirmed chiz's llama.cpp ignores the model name and honours `chat_template_kwargs.enable_thinking=false` (2-token reply, 0 reasoning). Added a `qwen3.8-27b-nothink` variant; **T4 glide dispatched to chiz with thinking off.** | Tests recommendation #4 directly: does a no-thinking dense 27B translate a precise spec faster without losing correctness? |
| 14:29:03 | **T4 (chiz, thinking off) wrote the whole glide module ~1 min after dispatch** (vs 21 min for T3 with thinking on). | — |
| 14:29:42 | T4 compiles: 6/11. Then **oscillates at 7/11 for ~1.5 min**: each edit fixed one edge case and broke another. | Without thinking it iterates against test output instead of reasoning about the spec. |
| 14:31 | Claude reads T4's code: four concrete misreadings (first chord given a slide; "previous" pitches read after being overwritten; inverted surplus-voice check; inactivity never cleared). **Retry with those four notes, same worktree.** | One round of specific notes instead of waiting. |
| 14:32:12 | **T8 sequencer (mini): 9/9**, about 40 s after its first edit. Merged after review (clean; minor O(n) walk in `boundaries`). | — |
| 14:32:57 | Claude's controller tests (keys → chords → voicings → loop scheduling) pass against the workers' merged crates. | First proof the pieces integrate. |
| 14:33:43 | **T4 retry: 11/11** about 2 min after the notes. Its last clippy error was in **Claude's test file** (`map_or(false, …)`); it rightly didn't edit it and reported clippy on `--lib` instead. Claude fixed the test lint. | Test files must pass clippy too; a worker can't fix them. |
| 14:34 | **Second review catch:** `lands_at` returned the first chord's onset whenever glide = 0, so MPE output would never re-strike. Test added (fails), one-line fix, merged. | Edge cases still slip past thinking-off. |
| 14:34 | **You:** make the local machines do as much as possible; Claude only checks and reports. | Workers now write their own tests; Claude writes short specs and reviews. |
| 14:35 | T5a moved from the mini queue to chiz (thinking off); the mini's pending retry cancelled. Specs written for T9 (MIDI export), T10 (chord recognition) and T12 (17 more sounds), all with worker-written tests. | — |
| 14:36:40 | T5a (chiz) wrote `smf.rs`; it can't go green because **Claude's verify command also ran the MPE stub tests**, which were added after the spec was written. Spec fixed; T5a restarted in its worktree. | Spec drift: a verify command must only cover the task's own tests. |
| 14:37 | **Collateral damage:** `pkill -f "T5a: midi::smf"` also matched the mini's queue script (that text was in its command line) and killed it. T6's opencode process survived; T5b's automatic start was lost and is now re-queued, waiting on **T6's PID**. | Kill and wait by PID, never by pattern. |

## Token ledger: delegating vs doing it myself

**Local tokens are measured** from opencode's database (`message.tokens`). These llama.cpp providers report reasoning inside `output`. **Claude's tokens are estimates:** Claude can't meter its own usage, so they're derived from lines written (about 12 output tokens per line of Rust/Markdown) and from counting tool calls. Snapshot at 14:24.

### Local machines (free at the margin, but wall-clock and supervision aren't)

| Run | Machine | Output tok (incl. reasoning) | Prompt tok (new + cache read) | Useful result |
|---|---|---|---|---|
| T1 | mini | 7.4k | 42k + 298k | 134 lines, merged |
| T2 (killed while 3 shared the mini) | mini | 0.2k | 38k | none |
| T2 | mini | 18.7k | 14k + 917k | 106 lines, merged after review fix |
| T5a (killed) | mini | 0.1k | 17k | none |
| T3 #1 | chiz | **32.4k** | 23k + 61k | **0 lines: all reasoning, hit the cap** |
| T3 #2 (in progress) | chiz | 32.1k+ | 39k + 276k | ~95 lines so far, 3/9 tests |
| **Total** | | **~91k** | **~1.7M** | **~335 lines** |

Waste: **~32.7k local output tokens (36%)** produced nothing: T3 #1 plus the killed runs.

### Claude (the paid side)

| Item | Est. Claude output tokens | Note |
|---|---|---|
| **Saved:** T1 and T2 bodies not written by Claude | **−2.9k** | 240 lines at ~12 tok/line |
| **Saved:** T3 body | 0 | Claude wrote a reference anyway (+1.8k), so delegating T3 saved nothing so far |
| **Spent:** task specs + dispatch script (incl. rewrites) | +4.5k | Needed only because the work was delegated |
| **Spent:** reviews (T1, T2), merge and worktree management | +1.5k | |
| **Spent:** diagnosing infra (slot contention, output cap, stale worktree, script crash) and status updates | +6k | About 45 extra tool calls |
| **Net output** | **≈ +11k spent vs ~3k saved** | |

There's also a larger, hidden cost: **every Claude tool call re-reads the conversation context** (mostly a cheap cache hit, but large). The ~45 supervision and diagnosis calls above add roughly **45 × the current context size** in cached input tokens, an order of magnitude more than the output figures.

**Verdict so far: delegation is net negative on Claude tokens (≈4× more spent than saved) and slower in wall-clock.** It did produce two correct modules, a lot of signal about the models, and it surfaced harness problems that are now fixed. The skeletons and tests (~1,650 lines) aren't counted as overhead; they're how Claude would build this anyway.

## How to get offloading to pay off

The ledger shows the break-even condition: **(implementation tokens the worker saves Claude) must exceed (spec + review + supervision tokens Claude spends).** Today's tasks failed it because they were small (a table, a key map) and needed babysitting. Changes, in order of impact:

1. **Supervision off the critical path.** Most of Claude's cost was watching and diagnosing. A queue runner plus an automated gate (build, test, clippy, scope and test-integrity checks, token logging) should run tasks with no Claude involvement. Claude reviews each *merged batch* once, in one pass. That removes most of the ~45 extra calls.
2. **Smaller, translation-shaped tasks.** One function per task, about 60 lines or less, each with its own tests. T3 should have been four tasks: `bass_note` + `motion`, `candidates`, `next`, `voice_cycle`.
3. **Claude writes pseudo-code, the worker types Rust.** For algorithmic code, Claude puts the algorithm in the doc comment as numbered steps (cheap: ~20 lines). The local model only translates. This moves the expensive *thinking* to Claude (short) and the *typing* to the worker (long).
4. **Turn off thinking for translation tasks.** Qwen-family templates accept `chat_template_kwargs: {"enable_thinking": false}` (your global config already uses `chat_template_kwargs` for Bonsai). Defining a non-thinking model variant in `opencode.json` for spec-to-code work would have avoided T3 #1's 32k-token empty reply.
5. **Route by observed behaviour, not by model size.**
   - The **mini** writes immediately and iterates with the compiler. That's ideal for test-driven, mechanical tasks.
   - **chiz** deliberates. Use it with thinking off for translation, or with thinking on for large-context reading (cross-file refactors, writing tests from a spec, reviewing a diff), where long reasoning earns its keep.
6. **Specs tuned to each model's weaknesses:** `\u{}` escapes for non-ASCII (mini); enum variants by name, never by index; pin down edge cases the obvious tests miss, such as a non-C split.
7. **Measure every task automatically.** `dispatch.sh` should append each session's token counts to a ledger file, so this table maintains itself.

## Summary

_Written once both queues have run._
