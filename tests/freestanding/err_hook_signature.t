// The runtime calls the platform's functions as it declares them: a byte
// at a time.
extern fn t_platform_write(b: u32) {} // error: the platform's `t_platform_write` is a `fn(u8)`, as the runtime calls it

fn main() {}
