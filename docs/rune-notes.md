# Rune cheat sheet (pinned rev 2a44156)

Everything a keychords UI task needs. Workers: this file is inside the repo on purpose. Do **not** read files outside your worktree (including `~/.cargo`); the headless sandbox refuses it and your run ends.

## Building elements
```rust
use rune::{canvas, col, el, row, text, Color, El, LineCap, Path, Phase, Shape, TextStyle};
col()            // children stacked vertically
row()            // children side by side
el()             // a box; SIBLINGS INSIDE el() ARE LAID OUT IN A ROW (not stacked)
text("hi").font(TextStyle::new(17.0, 700.0)).color(c)
```
To layer one thing on another, **nest** it (the face is the only child of the edge) or use `.absolute(x, y)` on the child. Never put two full-size siblings in one `el()`.

Common builder methods: `.size(w, h)`, `.w(w)`, `.h(h)`, `.w_full()`, `.h_full()`, `.grow(1.0)`, `.padding(p)`, `.px(p)`, `.gap(g)`, `.mt(m)`, `.ml(m)`, `.radius(r)`, `.fill(color)`, `.border(width, color)`, `.opacity(a)`, `.center()`, `.items_center()`, `.justify_between()`, `.clip_rounded(r)`, `.child(el)`, `.key("id")`.

## Colour
```rust
rune::color::hex("#E7E4F1").unwrap()      // -> Color
rune::oklch(l, c, hue_degrees)            // perceptual colour
Color { a: 0.5, ..col }                   // alpha
```
Use the tokens and helpers in `crates/app/src/theme.rs`.

## Input
```rust
.on_press(|s: &mut State| { /* click */ })
.on_drag(|s: &mut State, d: &mut rune::Drag, _tx| {
    // d.phase: Phase::Down | Phase::Move | Phase::Up
    // d.x, d.y: pointer in the element's own points; d.w, d.h: element size
    // d.dx, d.dy: travel since press; d.data: scratch f32 kept across the gesture
    let v = (d.x / d.w).clamp(0.0, 1.0);   // a horizontal slider value 0..1
})
.on_key_up(|s: &mut State, e: &rune::KeyEvent, tx: &mut rune::Tx| -> bool { true })
.focusable(true).autofocus()
```
Global keys arrive in `on_key(s, e, tx)` in `main.rs` (`tx.physical_key()` gives the position).

Example (Rune's sketch demo):
```rust
canvas(shape).w_full().grow(1.0)
    .on_drag(|s: &mut State, d, _| match d.phase {
        Phase::Down => s.strokes.push(vec![[d.x, d.y]]),
        _ => if let Some(st) = s.strokes.last_mut() { st.push([d.x, d.y]) },
    })
```

## Drawing
```rust
let shape = Shape::new()
    .line([x0, y0], [x1, y1], width, LineCap::Round, color)
    .stroke_path(&Path::new().move_to(a).line_to(b).cubic_to(c1, c2, p), width, LineCap::Round, color)
    .view_box(1000.0, 300.0);      // coordinates stretch to the element; stroke widths stay in points
canvas(shape).grow(1.0).h(260.0)
```
See `voice_canvas` in `crates/app/src/main.rs` for a full example.

## Frames and time
`poll(s, seconds, tx)` runs every frame (the app wakes every 15 ms); store `seconds` in `State` to animate (see `s.clock`).

## Headless check
`cargo run -q -p app -- --snap logs/x.png --scene demo` renders one frame. You may not be able to view images; check structure in code instead.
