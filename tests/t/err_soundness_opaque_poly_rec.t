// Each call of `grow` is at the type the last returned, which holds the
// last's: the copies monomorphisation would make never end.
fn wrap<T: Discard>(x: T) -> impl Discard { x }

fn grow<T: Discard>(x: T, n: i64) -> i64 with panic {
    if n == 0 { 0 } else { grow(wrap(x), n - 1) }
}

fn main() with panic { print(grow(1i64, 3)); }
// error: polymorphic recursion
