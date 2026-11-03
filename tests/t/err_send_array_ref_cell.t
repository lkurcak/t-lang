// An array is its values: one of references to a cell isn't `Send`.
use core::cell::Cell;

fn send<T: Send + Discard>(_t: T) {}

fn main() {
    let c = Cell::new(1u8);
    send([&c, &c]);
// error: `send` needs its `T` to implement `Send`, and `[&Cell<u8>; 2]` doesn't
}
