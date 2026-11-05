// Also in what it calls: here, `unwrap` of `None`.
fn first(a: &[u8]) -> Option<u8> {
    match a.get(0) {
        Some(x) => Some(*x),
        None => None,
    }
}

const EMPTY: &[u8] = b"";
const HEAD: u8 = first(EMPTY).unwrap();
// error: evaluating `HEAD` panicked: `unwrap` of `None`

fn main() {
    let _ = HEAD;
}
