// Nor does passing it where one is wanted.
struct Fd { n: i64 }

fn run(k: fn() -> !) -> i64 { k() }

fn main() {
    let _fd = Fd { n: 1 };
    print(run(|| return 1));
}
// error: expected `!`, found `{integer}`
