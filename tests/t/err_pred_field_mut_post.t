// A result's refinement names what's behind a `&mut` at return only of a
// call-scoped parameter: here the borrow outlives the call.
#[derive(Discard)]
struct C { n: u8 }

fn f<'a>(c: &'a mut C) -> { r: &'a mut u8 | c.n == 0 } { &mut c.n }

fn main() {}
// error: the result's refinement can't name `c` at return: the borrow it's lent can outlive the call, as the lifetime of `c` is in the signature elsewhere; write `old(..)`
