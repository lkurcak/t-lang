// What a `break` gives a `loop` is the loop's value, so returning the loop
// returns it.
fn make(n: int) -> fn(int) -> int {
    loop {
        break |x| x + n;
    }
}
fn main() {
    print(make(1)(2));
}
// error: cannot return a closure that captures `n`
