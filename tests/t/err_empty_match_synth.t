enum Never {}
fn f(n: Never) {
    let x = match n {};
}
fn main() {}
// error: a `match` with no arms needs a known type here
