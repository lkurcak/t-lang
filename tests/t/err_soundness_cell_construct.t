// A cell is made by `Cell::new`, not by its field.
use core::cell::Cell;

fn main() {
    let c = Cell { value: 1 };
}
// error: private
