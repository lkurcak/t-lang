// A type parameter is `Sync` only if a bound says so.
fn sync<T: Sync>(_t: &T) {}

fn any<T>(t: &T) { sync(t); }
// error: `sync` needs its `T` to implement `Sync`, and `T` doesn't; add a bound: `T: Sync`

fn main() {}
