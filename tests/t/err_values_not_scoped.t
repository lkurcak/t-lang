// What a `&mut` whose borrow can outlive the call points to at return
// can't be named, as for its fields (docs/85-values-in-predicates.md).
fn keep<'a>(a: &'a mut u8) -> &'a mut u8 ensures *a == old(*a) { a }

fn main() {}
// error: `ensures` can't name `a` at return: the borrow it's lent can outlive the call
