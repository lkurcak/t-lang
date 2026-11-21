// Code generic in an error type can't know it's `!`: `unwrap` may panic.
fn get<E: Discard>(r: Result<i64, E>) -> i64 with {} { r.unwrap() }

fn main() with {} {
    let _v = get::<!>(Ok(1));
}
// error: `unwrap` may panic, but `get` doesn't say `with panic`
