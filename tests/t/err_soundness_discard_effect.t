// What's dropped is dropped everywhere, on every path out: a `discard`
// that could panic or allocate would add that to every function.
struct Loud { n: i64 }

impl Discard for Loud {
    fn discard(self) with panic {
        if self.n > 0 { core::panic::panic("loud") }
    }
}

fn main() {}
// error: `discard` may panic, but `Discard`'s `discard` can't, so its impls can't either
