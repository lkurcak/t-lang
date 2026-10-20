// What an opaque type hides can't do more than its bound says: callers of
// `make` see only `Fn(int) -> int`, which can't panic.
fn make() -> impl Fn(int) -> int { |x| x / 0 }

fn main() {
    let f = make();
    print(f(1));
}
// error: this closure may panic, but `make's return type` needs calls of it to fit `fn(int) -> int`, which can't panic
