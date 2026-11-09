// Functions can't be compared, so neither can tuples holding them.
fn g() {}

fn main() {
    let a = (g, 1i64);
    let _same = a == a;
}
// error: cannot compare `(fn() -> (), i64)` with `==`: that needs `fn() -> (): Eq`, and functions can't be compared
