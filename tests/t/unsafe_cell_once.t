// A cell set once, then read through `&`s for as long as it lives, written
// over `Unsafe` in the program itself: the compiler knows nothing of it.
use core::cell::{Cell, Unsafe};

#[derive(Discard where T: Discard)]
struct Once<T> { set: Cell<bool>, value: Unsafe<Option<T>> }

impl<T: Discard> Once<T> {
    fn new() -> Once<T> { Once { set: Cell::new(false), value: Unsafe::new(None) } }

    /// What's in it, if it's been set.
    fn get(&self) -> Option<&T> {
        if !self.set.get() {
            return None;
        }
        // SAFETY: once set, it's never lent as `&mut` again.
        let value = unsafe { self.value.as_ref() };
        value.as_ref()
    }

    /// Sets it to `v`, if it hasn't been: `Err(v)` if it has.
    fn set(&self, v: T) -> Result<(), T> {
        if self.set.get() {
            return Err(v);
        }
        // SAFETY: it hasn't been set, so no `&` from `get` lives, and the
        // `&mut` lives only for this assignment, which drops a `None`.
        unsafe {
            *self.value.as_mut() = Some(v);
        }
        self.set.set(true);
        Ok(())
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let once: Once<i64> = Once::new();
    let mut out = os.stdout;
    out.line(&once.get().is_none())?; // expect: true
    out.line(&once.set(7).is_ok())?;  // expect: true
    let r = once.get();
    out.line(&once.set(8).is_err())?; // expect: true
    match r {
        Some(v) => out.line(v)?, // expect: 7
        None => {},
    }
    Ok(())
}
