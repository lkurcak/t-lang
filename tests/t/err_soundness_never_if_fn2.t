// Whichever comes first.
struct Fd { n: int }

fn die() -> ! {
    loop {}
}

fn one() -> int { 1 }

fn main() {
    let f = if true { die } else { one };
    let _fd = Fd { n: 1 };
    f();
}
// error: `if` and `else` have different types: `fn() -> !` and `fn() -> int`
