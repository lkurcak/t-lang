// A stack of ints on a `RawBuf`, which keeps the obligations of the
// buffer's operations: slots `0..len` hold values, the rest don't.
use alloc::raw::RawBuf;
use alloc::string::String;
use core::fmt::Write;

struct Stack { buf: RawBuf<i64>, len: usize }

impl Stack {
    fn new() -> Stack with alloc { Stack { buf: RawBuf::new(0), len: 0 } }

    fn push(&mut self, v: i64) with {alloc, panic} {
        let cap = self.buf.cap();
        if self.len >= cap {
            self.buf.grow(self.len - cap + 1);
        }
        // SAFETY: `len < cap()`, and slot `len` is past those in use.
        unsafe { self.buf.write(self.len, v) };
        self.len = self.len + 1;
    }

    fn pop(&mut self) -> Option<i64> {
        if self.len == 0 || self.len > self.buf.cap() {
            return Option::None;
        }
        self.len = self.len - 1;
        // SAFETY: slot `len` was in use, and isn't any more.
        Option::Some(unsafe { self.buf.read(self.len) })
    }

    fn top(&self) -> Option<&i64> {
        if self.len == 0 || self.len > self.buf.cap() {
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
fn bump_first(b: &mut RawBuf<i64>) with {panic, unsafe} ensures b.cap() == old(b.cap()) {
    if b.cap() > 0 {
        *b.slice_mut(0, 1).get_mut(0).unwrap() += 1;
    }
}

/// Slot `i`'s value, moved out, if there's a slot `i`.
fn get(b: &mut RawBuf<i64>, i: usize) -> i64 with unsafe {
    if i < b.cap() { b.read(i) } else { 0 }
}

fn apply<effect e, F: FnOnce() -> i64 with e>(f: F) -> i64 with e { f() }

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = String::new();
    let mut s = Stack::new();
    let mut i = 0;
    while i < 10 {
        s.push(i * i);
        i += 1;
    }
    out.line(&*s.top().unwrap()); // expect: 81
    out.line(&s.pop().unwrap());  // expect: 81
    out.line(&s.pop().unwrap());  // expect: 64
    out.line(&s.len);             // expect: 8
    // SAFETY: slot 0 holds the first value pushed.
    unsafe { bump_first(&mut s.buf) };
    // SAFETY: the closure reads slot 7, which holds a value, and `pop`
    // isn't called after it until `len` says slot 7 is free.
    let last = unsafe { apply(|| get(&mut s.buf, 7)) };
    s.len = 7;
    out.line(&last); // expect: 49
    while s.len > 1 {
        s.pop();
    }
    out.line(&s.pop().unwrap()); // expect: 1
    s.free();
    let mut stdout = os.stdout;
    let w = stdout.put(&out.as_str());
    out.discard();
    w
}
