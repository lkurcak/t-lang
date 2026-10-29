// An alias of a function type without `with` stands for a total one: a
// function that can panic isn't one.
type Total = fn(i64) -> i64;

fn halve(n: i64) -> i64 with panic {
    if n % 2 != 0 { panic("odd") }
    n / 2
}

fn main() {
    let _f: Total = halve;
}
// error: expected `fn(i64) -> i64`, found `fn(i64) -> i64 with panic`
