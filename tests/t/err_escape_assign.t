// Assigning a closure to a variable from further out could let it outlive
// what it captures.
fn main() {
    let mut f = |x: int| x;
    let n = 1;
    f = |x| x + n;
    print(f(1));
}
// error: cannot assign a closure that captures `n`
