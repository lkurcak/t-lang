// `..` matches anything, so what the named values don't cover is missing.
enum E {
    Two(u8, bool),
}

fn f(e: E) -> u8 {
    match e {
        E::Two(0, ..) => 0,
    }
}

fn main() {
    let _ = f(E::Two(1, true));
}
// error: non-exhaustive `match`: `E::Two(1.., _)` not covered
