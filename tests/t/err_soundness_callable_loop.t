// A closure made in a loop is a new value each time round: one kept from
// the iteration before isn't the one this iteration made, though it's of
// the same type (exercise 2 of docs/74).
fn call<R, F: FnOnce() -> R + Discard>(f: F) -> R { f() }

fn main() with panic {
    let mut kept = None;
    for n in 0..9usize {
        if let Some(g) = kept {
            let x: { v: usize | v == n } = call(g);
            let _ = x;
        }
        let f = move || n;
        kept = Some(f);
    }
}
// error: can't prove `v == n` for `x`
