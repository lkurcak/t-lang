// An impl of `Discard` with nothing in it says nothing derive doesn't: the
// fix derives it.
struct Plain { n: i64 }

impl Discard for Plain {}

fn main() {
    let _p = Plain { n: 1 };
}
// error: an impl of `Discard` gives a `discard` of its own; for one that drops each field, derive it, with `#[derive(Discard)]`
