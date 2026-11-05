// A radix is from 2 to 36: the digits `0` to `9` and `a` to `z`.
fn main() {
    let _ = 'a'.to_digit(37);
}
// error: can't prove `r >= 2 && r <= 36` for argument `radix` of `to_digit`
