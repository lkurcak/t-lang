// A tuple holding what isn't `Discard` isn't either: it must be taken
// apart and its values used up.
struct Fd { n: i64 }

fn main() {
    let _t = (Fd { n: 1 }, 2i64);
}
// error: `_t` is never used up, and its type, `(Fd, i64)`, isn't `Discard`; to let it be thrown away, add `#[derive(Discard)]` to `Fd`
