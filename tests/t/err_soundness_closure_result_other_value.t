// A closure written in the call is summarized, but the callee is also given
// another closure of its type, made on the turn before, with what it
// captured then: what the callee's `R` is isn't only what the closure
// written in the call gives (since 0.89.0; lemma V5 of docs/78).
fn h<R, F: Fn() -> R + Discard>(f: F, other: Option<F>) -> (R, F) {
    match other {
        Some(g) => (g(), f),
        None => {
            let r = f();
            (r, f)
        },
    }
}

fn main() with panic {
    let mut kept = None;
    for n in 0..4usize {
        let (r, back) = h(move || n, kept);
        let x: { v: usize | v == n } = r;
        let _ = x;
        kept = Some(back);
    }
}
// error: can't prove `v == n` for `x`
