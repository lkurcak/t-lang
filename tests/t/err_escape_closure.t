// A closure's environment lives in the frame of the call that made it, here
// a call of `make`, which is over before `make()` returns.
fn main() {
    let b = 2;
    let make = || || b;
    print(make()());
}
// error: cannot return a closure that captures `b`
