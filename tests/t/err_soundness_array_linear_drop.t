// An array of values that aren't `Discard` isn't either: dropping it would
// drop them.
struct Fd { n: i64 }

fn main() {
    let _fds = [Fd { n: 1 }, Fd { n: 2 }];
}
// error: `_fds` is never used up, and its type, `[Fd; 2]`, isn't `Discard`; to let it be thrown away, add `#[derive(Discard)]` to `Fd`
