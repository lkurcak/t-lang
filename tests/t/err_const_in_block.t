// A const is an item of a module, not of a block.
fn main() {
    const N: u8 = 3;
// error: a const is an item of a module
    let _ = N;
}
