// A list of our own whose `get` is off by one: it reads the slot at `len`,
// which holds nothing. The checker trusts the `unsafe` block; the
// interpreter catches it.
use alloc::raw::RawBuf;

struct Bad { buf: RawBuf<int>, len: { l: int | l >= 0 } }

impl Bad {
    fn get(&self, i: int) -> Option<&int> {
        if i >= 0 && i <= self.len && i < self.buf.cap() {
            // SAFETY: wrong: slot `len` isn't in use.
            unsafe { self.buf.slice(i, i + 1).get(0) }
        } else {
            Option::None
        }
    }
}

fn main() with {alloc, panic} {
    let mut b = Bad { buf: RawBuf::new(4), len: 0 };
    // SAFETY: slot 0 is uninitialised.
    unsafe { b.buf.write(0, 1) };
    b.len = 1;
    print(*b.get(0).unwrap()); // expect: 1
    print(*b.get(1).unwrap());
    // SAFETY: slot 0 holds the 1; then every slot is uninitialised.
    unsafe {
        b.buf.read(0);
        b.buf.free();
    }
}
// error: unsafe code broke its obligation: `RawBuf::slice` of slots 1..2, but slot 1 is uninitialised
