// A function type with no `with` can't panic, so a function that may
// doesn't fit it: calling `f` would panic in a `main` that says it can't.
fn inc(n: i64) -> i64 with panic { n + 1 }

fn main() {
    let f: fn(i64) -> i64 = inc;
    print(f(9223372036854775807));
}
// error: expected `fn(i64) -> i64`, found `fn(i64) -> i64 with panic`
