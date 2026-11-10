//! keychords standalone app (Rune UI).

mod controller;
mod engine;

use rune::tok::{self, type_};
use rune::{col, text, AppSpec, Cx, El};

#[derive(Default)]
struct State;

fn view(_: &State, _: &mut Cx<State>) -> El {
    let t = tok::theme(true);
    col().w_full().h_full().center().fill(t.bg)
        .child(text("keychords").font(type_::TITLE).color(t.text))
}

fn main() {
    rune::run(AppSpec::<State>::new("keychords", (960.0, 640.0), State::default, view));
}
