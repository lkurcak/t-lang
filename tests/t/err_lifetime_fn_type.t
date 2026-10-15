// A function that only takes `'static` references can't be used where one
// that takes any is wanted.
fn apply(f: fn(&int) -> int, x: &int) -> int { f(x) }
fn only_static(r: &'static int) -> int { *r }

fn main() {
    let n = 1;
    print(apply(only_static, &n));
}
// error: this function isn't general enough
