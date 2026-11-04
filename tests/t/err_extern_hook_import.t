// The runtime calls the platform's functions; a program can't call them.
extern fn t_platform_write(b: u8);
// error: `t_platform_write` is the platform's, which the runtime calls

fn main() {}
