// A library's own cell, a struct over an `Unsafe`, is invariant too, from
// its field alone: the compiler knows nothing about it.
use core::cell::Unsafe;

struct MyCell<T> { value: Unsafe<T> }

impl<T> MyCell<T> {
    fn put(&self, v: T) {
        // SAFETY: no loan of what's in it lives across this.
        unsafe {
            *self.value.as_mut() = v;
        }
    }
}

fn shorten<'a>(c: &'a MyCell<&'static str>) -> &'a MyCell<&'a str> { c }

fn main() {}
// error: lifetime may not live long enough
