// A cell of what can't be thrown away can't be thrown away either.
use core::cell::Cell;

struct Ticket { n: i64 }

fn main() {
    let c = Cell::new(Ticket { n: 1 });
}
// error: Discard
