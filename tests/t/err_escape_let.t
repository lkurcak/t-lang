// Through a variable, the closure is still the one that captures `n`.
fn adder(n: int) -> fn(int) -> int {
    let add = |x| x + n;
    add
}
fn main() { print(adder(5)(1)); }
// error: cannot return a closure that captures `n`
