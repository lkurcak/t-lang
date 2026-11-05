// A struct holding a mutex derives `Sync` from it: what that needs of `T`
// is `Send`, as the mutex's `unsafe impl` says, so a derived `Sync` can
// need `Send` of a type parameter.
use core::cell::Cell;
use core::sync::Mutex;

#[derive(Discard where T: Discard, Send where T: Send, Sync where T: Send)]
export struct Shared<T> { m: Mutex<T>, uses: core::sync::Atomic<u32> }

fn shared<T: Sync>(_t: &T) {}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    // A cell is `Send`, so a mutex of one, and this, are `Sync`.
    let s = Shared { m: Mutex::new(Cell::new(1u8)), uses: core::sync::Atomic::new(0) };
    shared(&s);
    let n = s.m.lock(|c: &mut Cell<u8>| c.replace(2));
    let mut out = os.stdout;
    out.line(&n)?; // expect: 1
    Ok(())
}
