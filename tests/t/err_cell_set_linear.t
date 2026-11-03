// `set` drops what was in the cell, so it needs `Discard`: `replace` is
// how to change a cell of what can't be thrown away.
use core::cell::Cell;

struct Ticket { n: i64 }

fn main() {
    let c = Cell::new(Ticket { n: 1 });
    c.set(Ticket { n: 2 });
    let Ticket { n } = c.into_inner();
}
// error: Discard
