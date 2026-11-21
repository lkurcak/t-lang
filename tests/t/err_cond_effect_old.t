// A condition is already of what the parameters are at the call: `old(..)`
// is for what's ensured.
fn f(l: &mut alloc::list::List<i64>) with panic if old(l.len()) == 0 { l.clear() }

fn main() {}
// error: `old(..)` can only be in a postcondition
