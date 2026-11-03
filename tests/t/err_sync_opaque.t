// What a function returns as `impl` is `Send` only if it says so.
fn send<T: Send>(_t: &T) {}

fn adder(n: i64) -> impl Fn(i64) -> i64 { move |x| if x < n { n } else { x } }

fn main() {
    let f = adder(1);
    send(&f);
// error: `send` needs its `T` to implement `Send`, and `impl Fn(i64) -> i64` doesn't
}
