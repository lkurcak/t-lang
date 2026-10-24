// `discard` is `Discard`'s: a value that isn't can't be thrown away by
// calling it either.
struct Fd { n: i64 }

fn main() {
    let f = Fd { n: 1 };
    f.discard();
}
// error: `Fd` has no method `discard`; trait `Discard` has one, but `Fd` doesn't implement it
