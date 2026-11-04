// On the hosted target, the platform is the C library.
extern fn t_platform_halt() -> ! { loop {} }
// error: `t_platform_halt` is the platform's, and on the hosted target the platform is the C library's

fn main() {}
