# T5a: midi::smf — Standard MIDI File reader/writer

Implement every `todo!()` in `crates/midi/src/smf.rs`: `write_vlq`, `read_vlq`, `Smf::to_bytes`, `Smf::parse`.

The module doc comment is the spec for byte layout; `crates/midi/tests/smf.rs` has exact byte fixtures. Read both first.

Reference:
- Chunks: "MThd" + u32 BE length 6 + u16 format + u16 ntracks + u16 ppq; then "MTrk" + u32 BE length + events.
- Event = VLQ delta + message. Channel messages: 0x80 note off, 0x90 note on, 0xB0 CC, 0xE0 pitch bend (LSB then MSB, 7 bits each), low nibble = channel. 0xC0/0xD0 take 1 data byte, 0xA0 takes 2: parse and skip them. Meta: 0xFF type VLQ-length data. SysEx: 0xF0 or 0xF7, VLQ length, data: skip.
- Running status: a data byte (< 0x80) where a status is expected reuses the previous channel status.

## Files you may edit
- crates/midi/src/smf.rs

## Verify
```
cargo test -p midi && cargo clippy -p midi --all-targets -- -D warnings
```

## Rules
- Only edit the files listed under "Files you may edit". Do not touch tests, other crates, Cargo.toml files, or add dependencies.
- Never edit anything under `tests/`. The tests are the spec; if one looks wrong, say so in your final message instead of changing it.
- Replace every `todo!()` in your files with a real implementation. Keep the public signatures exactly as they are.
- Keep the style of the surrounding code: plain, idiomatic Rust, short doc comments only where they add something.
- Loop until the verification command passes with zero failures and zero clippy warnings.
- When done, reply with: what you implemented, the final test output summary line, and anything you think is wrong in the spec.
