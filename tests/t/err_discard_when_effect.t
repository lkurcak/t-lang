// A refined type's `discard` is `Discard`'s, which can't have effects: it
// runs wherever a value is dropped.
struct Fd { n: i64 }

struct Bag { n: i64, fd: Fd }

impl Discard for { b: Bag | b.n == 0 } {
    fn discard(self) with panic {
        let Bag { n: _, fd } = self;
        let Fd { n: _ } = fd;
        panic("boom");
    }
}

fn main() {}
// error: `discard` may panic, but `Discard`'s `discard` can't, so its impls can't either
