// What a closure's body works out of a value from before it, a measure of
// a list it borrows, is the same in every run: what it gives back can be
// said of it (docs/74).
use alloc::list::List;

fn call<R, F: FnOnce() -> R + Discard>(f: F) -> R { f() }

fn size(l: &List<u8>) -> usize {
    let n: { v: usize | v == l.len() } = call(|| l.len());
    n
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let l: List<u8> = List::filled(3, 0);
    out.line(&size(&l))?; // expect: 3
    Ok(())
}
