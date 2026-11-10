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
| T2 keymap | mini | mechanical, some logic | pending | | | | | | | |
| T5a SMF | mini | byte-level, spec-heavy | pending | | | | | | | |
| T3 voicing | chiz | algorithmic | pending | | | | | | | |

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

## Summary

_Written once both queues have run._
