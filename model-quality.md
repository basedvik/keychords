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
| T1 chords | mini | mechanical | pending | | | | | | | |
| T2 keymap | mini | mechanical, some logic | pending | | | | | | | |
| T5a SMF | mini | byte-level, spec-heavy | pending | | | | | | | |
| T3 voicing | chiz | algorithmic | pending | | | | | | | |

## Per-task notes

_Filled in as each task is gated._

## Summary

_Written once both queues have run._
