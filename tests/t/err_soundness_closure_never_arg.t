// Nor does passing it where one is wanted.
struct Fd { n: int }

fn run(k: fn() -> !) -> int { k() }

fn main() {
    let _fd = Fd { n: 1 };
    print(run(|| return 1));
}
// error: expected `!`, found `int`
