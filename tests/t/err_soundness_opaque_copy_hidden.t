// The closure only holds an `int`, so it's `Copy`, but the return type
// doesn't say so: callers can't copy it.
fn adder(n: int) -> impl Fn(int) -> int + Discard { move |x| x + n }

fn twice<F: Copy>(f: F) -> F {
    let _g = f;
    f
}

fn main() { print(twice(adder(1))(2)); }
// error: `twice` needs its `F` to implement `Copy`, and `impl Fn(int) -> int + Discard` doesn't
