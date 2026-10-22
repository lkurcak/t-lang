// What `main` returns is shown, and then thrown away at the end, as
// nothing can use it up. So it must be `Discard`, or a file descriptor, or
// a buffer, would leak.
struct Fd { n: i64 }

fn main() -> Fd { Fd { n: 1 } }
// error: `main` can't return a `Fd`, which isn't `Discard`
