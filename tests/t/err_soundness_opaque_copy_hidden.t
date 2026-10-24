// The closure only holds an `i64`, so it's `Copy`, but the return type
// doesn't say so: callers can't copy it.
fn adder(n: i64) -> impl Fn(i64) -> i64 with panic + Discard { move |x| x + n }

fn twice<F: Copy>(f: F) -> F {
    let _g = f;
    f
}

fn main(os: std::Os) -> Result<(), std::io::Error> { let mut out = os.stdout; out.line(&twice(adder(1))(2))?;  Result::Ok(()) }
// error: `twice` needs its `F` to implement `Copy`, and `impl Fn(i64) -> i64 with panic + Discard` doesn't
