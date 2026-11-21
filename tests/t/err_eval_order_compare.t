// `==` looks at its operands where they are, and at its left one as it is
// where it's written: borrowed there, if it isn't `Copy`, so the right
// can't change it (docs/91-evaluation-order.md).
use alloc::list::List;

fn main() with alloc {
    let mut a: List<u8> = List::new();
    let b: List<u8> = List::new();
    let _same = a == { a.push(1); b };
    a.discard();
}
// error: cannot borrow `a` as mutable while `a` is borrowed
