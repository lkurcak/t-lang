// A reference to a cell can't go to another thread: both could change it
// at once.
use core::cell::Cell;

fn main(os: std::Os) with alloc {
    let c = Cell::new(1u32);
    let h = os.threads.spawn(|| c.set(2));
// error: `Threads::spawn` needs its `F` to implement `Send`, and `{closure}` doesn't; that needs `Cell<u32>: Sync`
    h.join();
    c.set(3);
}
