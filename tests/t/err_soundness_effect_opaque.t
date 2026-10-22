// What an opaque type hides can't do more than its bound says: callers of
// `make` see only `Fn(i64) -> i64`, which can't panic.
fn make() -> impl Fn(i64) -> i64 { |x| x / 0 }

fn main() {
    let f = make();
    print(f(1));
}
// error: this closure may panic, but `make's return type` needs calls of it to fit `fn(i64) -> i64`, which can't panic
