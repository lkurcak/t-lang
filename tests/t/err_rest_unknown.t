// How many values `..` stands for is the type's to say: here the type
// isn't known where the pattern is.
fn main() {
    let f = |(a, ..)| a + 1u8;
    let _ = f((1u8, 2u8));
}
// error: the type of this tuple isn't known here
