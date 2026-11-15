// A closure given for `F` beside an iterator whose items are of its type:
// the callee may call one of those (lemma V5 of docs/78).
use alloc::list::List;

fn first<R, F: FnOnce() -> R + Discard, I: Iterator<Item = F> + Discard>(f: F, mut it: I) -> R {
    match it.next() {
        Some(g) => g(),
        None => f(),
    }
}

fn main() with {alloc, panic} {
    let mut fs = List::new();
    for n in 0..4usize {
        let f = move || n;
        let x: { v: usize | v == n } = first(f, fs.iter().copied());
        let _ = x;
        fs.push(f);
    }
}
// error: can't prove `v == n` for `x`
