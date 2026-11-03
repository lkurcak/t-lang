// A derived `Send` of a struct holding a `&T` needs `T: Sync`, not `T:
// Send`: a cell is `Send`, but a reference to one handed over would let two
// threads change it.
use core::cell::Cell;

#[derive(Discard, Send)]
struct Holder<'a, T> { r: &'a T }

fn send<T: Send + Discard>(_t: T) {}

fn main() {
    let c = Cell::new(1u8);
    send(Holder { r: &c });
// error: `send` needs its `T` to implement `Send`, and `Holder<Cell<u8>>` doesn't
}
