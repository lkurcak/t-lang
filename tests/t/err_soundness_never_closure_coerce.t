// Nor is a closure that returns.
struct Fd { n: int }

fn main() {
    let f = || 1;
    let g: fn() -> ! = f;
    let _fd = Fd { n: 1 };
    g();
}
// error: expected `fn() -> !`, found `fn() -> int`
