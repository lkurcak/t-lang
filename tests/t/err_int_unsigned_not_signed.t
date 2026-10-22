// A `u8` is not `Signed`.
fn f<T: Signed>(x: T) -> T { x }

fn main() {
    print(f(1u8));
}
// error: `f` needs its `T` to implement `Signed`, and `u8` doesn't
