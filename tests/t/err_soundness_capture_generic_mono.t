// Capturing a `T: Copy` checks in the generic function, and the copy for
// `T` = `&int` is then rejected: a closure can't hold a reference. That's
// sound, but whether a use is accepted depends on monomorphisation, not on
// the signature. (Every value of type `T` outlives the function's body, so
// the capture is safe; closures with types that say what they capture
// would accept it.)
fn call<T: Copy>(x: T) -> T {
    let g = || x;
    g()
}

fn main() {
    let n = 5;
    print(*call(&n));
}
// error: cannot use `call` with `T` = `&int` here: in that copy of it, this closure can't capture `x`
