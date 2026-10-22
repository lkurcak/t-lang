// Nor is a closure that returns.
struct Fd { n: i64 }

fn main() {
    let f = || 1i64;
    let g: fn() -> ! = f;
    let _fd = Fd { n: 1 };
    g();
}
// error: expected `fn() -> !`, found `fn() -> i64`
