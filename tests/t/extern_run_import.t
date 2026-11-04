// What a C function does is the machine's: the interpreter stops where
// one is called.
extern fn putchar(c: i32) -> i32;

fn main() {
    // SAFETY: `putchar` is the C library's, and does nothing but write.
    let _ = unsafe { putchar(104) };
// error: `putchar` is a C function, which the interpreter can't call
}
