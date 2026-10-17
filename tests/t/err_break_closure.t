// A closure's body is a function of its own, so a `break` in it can't leave
// the loop the closure is made in: the closure may be called anywhere.
fn main() {
    loop {
        let f = || { break; };
        f();
    }
}
// error: can't `break` out of a closure
