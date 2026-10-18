// A type that holds what can't be thrown away can't be thrown away.
struct Fd { n: int }

#[derive(Discard)]
struct Holder { fd: Fd }

fn main() {}
// error: `Holder` can't be `Discard`: its field `fd` holds a `Fd`, which isn't
