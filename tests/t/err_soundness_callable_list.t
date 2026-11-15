// A closure given for `F` beside a list of others of its type: the callee
// may call one of those (lemma V5 of docs/78).
use alloc::list::List;

fn last<R, F: Fn() -> R + Discard>(f: F, fs: &List<F>) -> R {
    match fs.last() {
        Some(g) => g(),
        None => f(),
    }
}

fn main() with {alloc, panic} {
    let mut fs = List::new();
    for n in 0..4usize {
        let f = move || n;
        let x: { v: usize | v == n } = last(f, &fs);
        let _ = x;
        fs.push(f);
    }
}
// error: can't prove `v == n` for `x`
