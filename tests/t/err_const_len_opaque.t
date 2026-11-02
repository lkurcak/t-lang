// As a length, a const is equal only to itself: `[u8; N]` isn't `[u8; 3]`,
// so what a function's type says doesn't change with the const's value.
const N: usize = 3;

fn main() {
    let _a: [u8; N] = [1, 2, 3];
// error: expected `[u8; N]`, found `[u8; 3]`
}
