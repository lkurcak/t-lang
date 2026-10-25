// An array's length is part of its type: three values aren't two.
fn main() {
    let _a: [i64; 3] = [1, 2];
}
// error: expected `[i64; 3]`, found `[i64; 2]`
