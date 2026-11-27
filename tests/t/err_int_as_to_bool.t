fn main() {
    let x: u8 = 1;
    let _b = x as bool;
}
// error: `as` converts between integer and floating-point types and `char`, not to `bool`
