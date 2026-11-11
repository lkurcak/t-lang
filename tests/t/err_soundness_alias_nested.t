// An alias of an alias's refinement, in a type argument: each element of
// the list.
use alloc::list::List;

type Square = { s: u8 | s < 64 };
type Squares = List<Square>;

fn main() with alloc {
    let mut l: Squares = List::new();
    l.push(64);
}
// error: can't prove `s < 64`
