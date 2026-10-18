// A generic function's result is `!` only if its type arguments make it so.
struct Fd { n: int }

fn one() -> int { 1 }

fn call<T>(f: fn() -> T) -> T { f() }

fn main() {
    let _fd = Fd { n: 1 };
    let _x: ! = call(one);
}
// error: expected `!`, found `int`
