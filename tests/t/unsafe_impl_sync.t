// A lock of one's own: `Cell::as_mut` lends what's in a cell as `&mut`
// from a `&`, which is unsafe, as nothing else may reach the cell while it
// lives; the flag makes sure of that. So `Flagged<T>` is `Sync`, against
// its fields, as `unsafe impl` says: the impl is its writer's promise.
use core::cell::Cell;
use core::sync::Atomic;

#[derive(Discard where T: Discard, Send where T: Send)]
struct Flagged<T> { busy: Atomic<bool>, value: Cell<T> }

unsafe impl<T: Send> Sync for Flagged<T> {}

impl<T> Flagged<T> {
    fn new(value: T) -> Flagged<T> { Flagged { busy: Atomic::new(false), value: Cell::new(value) } }

    // Runs `f` if no other thread is, or gives back `false`.
    fn try_with<F: FnOnce(&mut T)>(&self, f: F) -> bool where F: Discard {
        match self.busy.swap(true) {
            true => false,
            false => {
                // SAFETY: `busy` was `false`, and is `true` until `f` is done:
                // no other `as_mut` of the cell meanwhile, and nothing else
                // reaches it.
                f(unsafe { self.value.as_mut() });
                self.busy.store(false);
                true
            },
        }
    }
}

fn shared<T: Sync>(_t: &T) {}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let f = Flagged::new(1u64);
    shared(&f);
    let ran = f.try_with(|x: &mut u64| *x = core::num::wrapping_add(*x, 41));
    let mut out = os.stdout;
    out.line(&ran)?; // expect: true
    out.line(&f.value.get())?; // expect: 42
    Result::Ok(())
}
