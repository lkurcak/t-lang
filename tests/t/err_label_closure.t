// The label is there, but outside the closure, which is a function of its
// own.
fn main() {
    'outer: loop {
        let f = || {
            loop {
                break 'outer;
            }
        };
        f();
    }
}
// error: can't `break` out of a closure
