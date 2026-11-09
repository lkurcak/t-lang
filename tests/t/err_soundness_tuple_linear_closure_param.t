// What a closure's pattern doesn't bind is dropped: a linear value can't
// be left behind by `_`.
struct Fd { n: i64 }

fn main() {
    let f = |(_, n): (Fd, i64)| n;
    let _ = f((Fd { n: 1 }, 2));
}
// error: isn't `Discard`
