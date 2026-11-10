# keychords: deployment plan for chiz and the Mac mini

The goal is to build a Rust re-implementation of the system in `architecture.md`. The work is split across chiz (main coder), the Mac mini (small-model worker), and Claude (planner, reviewer, integrator). It is meant to be done in one session.

---

## 1. Roles

| Who | Model | Capacity | Gets |
|---|---|---|---|
| **Claude** (this session) | Opus 5.5 | n/a | Workspace skeleton, public interfaces, failing acceptance tests, task specs, review, merging, integration. Also any task that fails twice. |
| **chiz** | qwen3.8-27b Q8, 262k context, ~71 tok/s | **1 slot**: one worker at a time | Algorithmic or ambiguous tasks: voice leading, glide scheduling, MPE, plugin wiring, host sync. |
| **Mac mini** | *new model: waiting on your config* | TBD (slots, context, tool-call reliability) | Tightly specified, mechanical tasks: data tables, byte-level encoders, test fixtures, docs, clippy cleanup. |

**Routing rule:** each machine gets its **own top-level `opencode run`**, and Claude does the routing. opencode's nested subagent handoff (chiz handing off to the mini) is not used. It timed out in testing, and this setup doesn't need it.

---

## 2. How tasks are made small-model-proof: contract first

Before any worker starts, Claude writes:

1. **A Cargo workspace** with one crate per subsystem, holding public types and function signatures only (`todo!()` bodies).
2. **Failing acceptance tests** in each crate. The tests are the spec, and the compiler plus `cargo test` act as the oracle.
3. **One task file per job** (`tasks/NN-name.md`) with:
   - the goal
   - the files the worker **may** touch (everything else is off-limits)
   - the exact verification command
   - the relevant excerpt of `architecture.md`, not the whole doc, to fit the mini's context
   - "do not edit tests"

A worker's job is always: **"make `cargo test -p <crate>` pass without touching anything outside these paths."**

### Workspace layout

```
keychords/
  Cargo.toml                  # workspace
  crates/
    chords/     # 24 qualities × 12 roots, intervals, names, IDs       (mini)
    keymap/     # computer-key → root/quality/utility; MIDI split decode (mini)
    voicing/    # bass placement + upper-voice least-motion voice leader (chiz)
    glide/      # per-voice ramps, strum, loop lookahead, sample-accurate (chiz)
    midi/       # SMF read/write, MPE allocation, RPN bend range        (mini + chiz)
    synth/      # 7-voice synth with per-voice pitch smoothing          (mini)
    sequencer/  # loop slots, host-transport sync                        (chiz)
    plugin/     # nih-plug: VST3 + CLAP + standalone, params, MIDI I/O   (chiz)
  tasks/                      # task specs, one per worker job
  opencode.json               # project-local providers and models (your global config is untouched)
  scripts/dispatch.sh         # preflight + launch + log capture
```

> **Format note:** nih-plug ships VST3, CLAP and standalone builds but **not AU**. The default is VST3 + CLAP + standalone; AU would need a separate wrapper later.

---

## 3. Task DAG and machine assignment

```
                ┌────────────── Claude: skeleton + tests + task specs ───────────────┐
                ▼                                                                    ▼
  chiz queue (critical path, 1 slot)                 mini queue (parallel, independent)
  ──────────────────────────────────                 ──────────────────────────────────
  T3 voicing: voice leader ◄── needs T1 types*        T1 chords: dictionary + tests
  T4 glide: scheduler, strum, lookahead               T2 keymap: US key map + MIDI split decoder
  T5b midi: MPE allocation + export semantics         T5a midi: SMF writer/reader (VLQ, meta, RPN)
  T7 plugin: nih-plug shell, params, MIDI I/O         T6 synth: saw/filter/env voice, pitch smoothing
  T8 sequencer: loop + host transport                 T9 docs, clippy -D warnings, extra fixtures
                └──────────────► Claude: integration, end-to-end test, standalone run ◄──┘
```

\* T3 depends only on the `chords` **types**, which are already in Claude's skeleton. So T3 can start while T1 fills in the table bodies.

### Notable acceptance tests

- **T3 voicing.** The screenshot ground truth from `architecture.md` §6.1 becomes the test. `Dm9 → G13 → Cmaj9 → A7♭9` must produce:
  - bass D3, G2, C3, A2
  - top voice held at E5
  - upper-voice motion of 1, 7 and 2 semitones
  - no voice crossing
- **T4 glide.** In loop mode the ramp ends exactly on the slot boundary sample. Strum offsets go bass first. The ramp is eased in the semitone domain.
- **T5a/T5b MIDI.** Exporting then importing round-trips exactly. Each track carries RPN 0 = 48 semitones. The MPE layout is lower zone, member channels 2–8.
- **T2 keymap.** The full 24-key chord map and MIDI split map from §4 resolve correctly (for example, "D below split + split+5 → Dm7").

---

## 4. Dispatch mechanics

For each task:

```
1. preflight     curl /health on the target; check /props alias == expected model
                 (the mini serves whatever is loaded, whatever model name the request asks for)
2. worktree      git worktree add ../kc-T3 -b task/T3
3. launch        Bash, run_in_background:
                 opencode run -m <provider/model> "$(cat tasks/T3-voicing.md)"   (cwd = worktree)
                 stdout and stderr go to logs/T3.log; timeout per task
4. gate (Claude) cargo build && cargo test -p <crate> && cargo clippy -p <crate> -- -D warnings
                 git diff --stat touches only allowed paths
                 tests/ unchanged (no weakened assertions)
5. outcome       pass → merge to main, remove worktree, start the next task on that machine
                 fail → one retry with Claude's review notes in the prompt
                 fail twice → reroute (mini → chiz, chiz → Claude)
```

- **Concurrency:** at most 2 workers at once, one per machine. A second chiz job would only queue behind the first.
- **Context budget:** task prompts stay small enough to fit the mini's window with room for reasoning. Each prompt carries the relevant spec excerpt, not the whole document.

---

## 5. What's blocked on the mini config

| Need from your config | Why |
|---|---|
| provider id + model id + baseURL | So `opencode.json` and `dispatch.sh` target it. |
| context and output limits | Sets the task-prompt budget and which tasks fit. |
| reasoning/interleaved field | So opencode parses the thinking correctly. |
| parallel slots | Decides whether the mini can run more than one worker. |

**Then a smoke test before assigning anything:** one tool-call curl, plus a dry run of T1. If the new model fails it, the mini keeps only T9 (docs and fixtures), and T1, T2, T5a and T6 move to the chiz queue behind T3.

---

## 6. Order of work

1. Claude: workspace skeleton, interfaces, failing tests, task files, `opencode.json` (chiz entry), `dispatch.sh`.
2. Start **T3 on chiz** right away. It's the critical path and doesn't wait on the mini.
3. Your mini config arrives → smoke test → start the mini queue at T1.
4. Run both queues; Claude gates and merges as each task finishes.
5. Claude: integration, end-to-end test (key press → voicing → glide → MIDI out and audio), standalone build.
