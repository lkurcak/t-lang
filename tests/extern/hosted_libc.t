// On the hosted target, the C library's functions can be imported: even
// ones its headers declare, as an import's C name is its own.
extern fn putchar(c: i32) -> i32;
extern fn labs(x: i64) -> i64;

fn main() {
    // SAFETY: `putchar` writes a byte to `stdout`, and `labs` of a
    // `long` that isn't the least is its absolute value; `int64_t` is a
    // `long` here.
    unsafe {
        let _ = putchar(104);
        let _ = putchar(105);
        let _ = putchar(labs(-10) as i32);
    }
}
// prints: hi
