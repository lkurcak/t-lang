// A function type with no `with` can't panic, so a function that may
// doesn't fit it: calling `f` would panic in a `main` that says it can't.
fn inc(n: int) -> int with panic { n + 1 }

fn main() {
    let f: fn(int) -> int = inc;
    print(f(9223372036854775807));
}
// error: expected `fn(int) -> int`, found `fn(int) -> int with panic`
