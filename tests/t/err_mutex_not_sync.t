// A mutex is `Sync` only if what's in it may go to another thread: a
// reference to a cell may not, as the cell could be changed from both.
use core::cell::Cell;
use core::sync::Mutex;

fn shared<T: Sync>(_t: &T) {}

fn main() {
    let c = Cell::new(1u32);
    let m = Mutex::new(&c);
    shared(&m);
    // error: `shared` needs its `T` to implement `Sync`, and `Mutex<&Cell<u32>>` doesn't
}
