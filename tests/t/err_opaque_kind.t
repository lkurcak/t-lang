// The closure gives away the `Fd` it owns, so it can only be called once,
// but the return type says it can be called any number of times.
struct Fd { n: int }

fn close(fd: Fd) -> int {
    let Fd { n } = fd;
    n
}

fn once(fd: Fd) -> impl Fn() -> int { move || close(fd) }

fn main() { print(once(Fd { n: 1 })()); }
// error: this closure is only `FnOnce`, as
