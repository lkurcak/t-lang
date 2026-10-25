// An array's length is known before the program runs: a literal or a
// const parameter, not a variable.
fn main() {
    let n: usize = 3;
    let _a = [0; n];
}
// error: an array's length must be an integer literal or a const parameter
