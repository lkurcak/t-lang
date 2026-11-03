// A closure is what it captures: one holding a reference to a cell isn't
// `Send`.
use core::cell::Cell;

fn send<F: Fn() + Send + Discard>(f: F) { f(); }

fn main() {
    let c = Cell::new(1u8);
    send(|| c.set(2));
// error: `send` needs its `F` to implement `Send`, and `{closure}` doesn't; that needs `Cell<u8>: Sync`
}
