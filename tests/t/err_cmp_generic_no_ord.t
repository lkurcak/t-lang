// A type parameter is ordered only if a bound says so.
fn less<T: Eq>(a: &T, b: &T) -> bool { a < b }

fn main() {}
// error: cannot compare `&T` with `<`: that needs `T: Ord`; add a bound: `T: Ord`
