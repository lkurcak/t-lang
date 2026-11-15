// The orphan rule holds for an impl of `Discard` for a refined type as for
// any other: neither `Discard` nor `List` is the program's.
use alloc::list::List;

struct Fd { n: i64 }

impl Discard for { l: List<Fd> | true } {
    fn discard(self) {
        self.free_with(|f| {
            let Fd { n: _ } = f;
        });
    }
}

fn main() {}
// error: `Discard` is `core`'s and `List` is `alloc`'s, so only `core` or `alloc` can implement one for the other; to implement it for a type of your own, wrap it in a struct
