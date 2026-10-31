// The same in a result's refinement, which is a postcondition too.
use alloc::list::List;

fn first(l: &mut List<i64>) -> { s: &mut [i64] | s.len() == l.len() } { l.as_mut_slice() }

fn main() {}
// error: the result's refinement can't name `l` at return: the borrow it's lent can outlive the call, as the lifetime of `l` is in the signature elsewhere; write `old(..)`
