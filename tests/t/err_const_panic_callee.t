// Also in what it calls: here, `unwrap` of `None`.
fn first(a: &[u8]) -> Option<u8> {
    match a.get(0) {
        Option::Some(x) => Option::Some(*x),
        Option::None => Option::None,
    }
}

const EMPTY: &[u8] = b"";
const HEAD: u8 = first(EMPTY).unwrap();
// error: evaluating `HEAD` panicked

fn main() {
    let _ = HEAD;
}
