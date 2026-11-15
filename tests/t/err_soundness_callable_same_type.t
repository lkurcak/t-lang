// Two closures of one type given for one `F`: the callee calls the second,
// whose summary isn't the first's (lemma V5 of docs/78).
fn pick2<R, F: FnOnce() -> R + Discard>(_a: F, b: F) -> R { b() }

fn main() with panic {
    let mut kept = None;
    for n in 0..4usize {
        let f = move || n;
        if let Some(g) = kept {
            let x: { v: usize | v == n } = pick2(f, g);
            let _ = x;
        }
        kept = Some(f);
    }
}
// error: can't prove `v == n` for `x`
