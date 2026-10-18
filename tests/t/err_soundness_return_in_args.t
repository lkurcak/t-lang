// `return` in an argument leaves the arguments before it behind, and a
// linear one isn't used up.
struct Fd { n: int }

fn close(f: Fd) -> int {
    let Fd { n } = f;
    n
}

fn two(a: Fd, b: int) -> int { close(a) + b }

fn g() -> int {
    two(Fd { n: 1 }, return 2)
}

fn main() {
    print(g());
}
// error: this value is never used up, and its type, `Fd`, isn't `Discard`
