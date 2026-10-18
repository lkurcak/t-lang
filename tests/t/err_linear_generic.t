// A `T` may be anything, so it can't be thrown away unless a bound says so.
fn forget<T>(x: T) {} // warning: unused variable `x`: use it up, as its type, `T`, isn't `Discard`

fn main() {
    forget(1);
}
// error: `x` is never used up, and its type, `T`, isn't `Discard`; to let it be thrown away, add a bound: `T: Discard`
