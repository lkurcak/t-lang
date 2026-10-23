fn f() -> Option<i64> {
    let x = 3i64?;
    Option::Some(x)
}

fn main() { let _ = f(); }
// error: `?` needs a `Result`, an `Option` or another type that implements `core::ops::Try`, and `i64` doesn't
