// A reference to a cell can't go to another thread: both could change it.
use core::cell::Cell;

fn send<T: Send + Discard>(_t: T) {}

fn main() {
    let c = Cell::new(1u8);
    send(&c);
// error: `send` needs its `T` to implement `Send`, and `&Cell<u8>` doesn't
}
