// A closure whose body is a `return` returns: its type is `fn() -> i64`,
// not `fn() -> !`, so calling it doesn't excuse `_fd`.
struct Fd { n: i64 }

fn main() {
    let k = || return 1i64;
    let _fd = Fd { n: 1 };
    k();
}
// error: `_fd` is never used up
