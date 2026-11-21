// Code generic in an error type can't know it's `!`: `unwrap` may panic,
// as it does here, of a `bool` error.
fn get<E: Discard>(r: Result<i64, E>) -> i64 with {} { r.unwrap() }

fn main() with {} {
    let _v = get::<!>(Ok(1));
    let _w = get::<bool>(Err(true));
}
// error: `unwrap` may panic, but `get` doesn't say `with panic`
