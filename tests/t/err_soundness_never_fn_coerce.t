// A function that returns isn't one that doesn't.
struct Fd { n: int }

fn one() -> int { 1 }

fn main() {
    let f: fn() -> ! = one;
    let _fd = Fd { n: 1 };
    f();
}
// error: expected `fn() -> !`, found `fn() -> int`
