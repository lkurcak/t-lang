// A const is an integer or a `bool`, which has no fields or measures (the
// checker used to stop here, on an `unreachable!`).
const C: usize = 3;

fn f(x: { x: usize | x < C.len() }) -> usize { x }

fn main() {
    let _ = f(1);
}
// error: `C` is a const of type `usize`, which has no fields or measures
