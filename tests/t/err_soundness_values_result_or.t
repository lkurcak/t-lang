// A result that's one of two lists is neither one, to its caller
// (docs/85-values-in-predicates.md).
use alloc::list::List;

fn pick(a: List<u8>, b: List<u8>, first: bool) -> { r: List<u8> | r == a || r == b } {
    if first { a } else { b }
}

fn main() with alloc {
    let a: List<u8> = List::new();
    let mut b: List<u8> = List::new();
    b.push(1);
    let r = pick(a, b, false);
    let _ = r[0];
}
// error: can't prove `i < self.len()`
