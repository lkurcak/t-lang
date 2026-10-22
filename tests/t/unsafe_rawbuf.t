// A stack of ints on a `RawBuf`, which keeps the obligations of the
// buffer's operations: slots `0..len` hold values, the rest don't.
use alloc::raw::RawBuf;

struct Stack {
    buf: RawBuf<int>,
    len: { l: int | l >= 0 },
}

impl Stack {
    fn new() -> Stack with {alloc, panic} {
        Stack { buf: RawBuf::new(0), len: 0 }
    }

    fn push(&mut self, v: int) with {alloc, panic} {
        if self.len == self.buf.cap() {
            self.buf.grow(self.len * 2 + 1);
        }
        // SAFETY: `len < cap()`, and slot `len` is past those in use.
        unsafe { self.buf.write(self.len, v) };
        self.len = self.len + 1;
    }

    fn pop(&mut self) -> Option<int> {
        if self.len == 0 {
            return Option::None;
        }
        self.len = self.len - 1;
        // SAFETY: slot `len` was in use, and isn't any more.
        Option::Some(unsafe { self.buf.read(self.len) })
    }

    fn top(&self) -> Option<&int> {
        if self.len == 0 {
            return Option::None;
        }
        // SAFETY: slot `len - 1` is in use.
        unsafe { self.buf.slice(self.len - 1, self.len).get(0) }
    }

    fn free(mut self) {
        while self.pop().is_some() {}
        // SAFETY: popping everything left every slot uninitialised.
        unsafe { self.buf.free() };
    }
}

/// Unsafe itself: the caller must know slot 0 holds a value.
fn bump_first(b: &mut RawBuf<int>) with {panic, unsafe} {
    *b.slice_mut(0, 1).get_mut(0).unwrap() += 1;
}

fn apply<e, F: FnOnce() -> int with e>(f: F) -> int with e { f() }

fn main() with {alloc, panic} {
    let mut s = Stack::new();
    let mut i = 0;
    while i < 10 {
        s.push(i * i);
        i += 1;
    }
    print(*s.top().unwrap()); // expect: 81
    print(s.pop().unwrap());  // expect: 81
    print(s.pop().unwrap());  // expect: 64
    print(s.len);             // expect: 8
    // SAFETY: slot 0 holds the first value pushed.
    unsafe { bump_first(&mut s.buf) };
    // SAFETY: the closure reads slot 7, which holds a value, and `pop`
    // isn't called after it until `len` says slot 7 is free.
    let last = unsafe { apply(|| s.buf.read(7)) };
    s.len = 7;
    print(last); // expect: 49
    while s.len > 1 {
        s.pop();
    }
    print(s.pop().unwrap()); // expect: 1
    s.free();
}
