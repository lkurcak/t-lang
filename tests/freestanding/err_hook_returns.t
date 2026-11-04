// What halts never returns: its type says so.
extern fn t_platform_halt() {} // error: the platform's `t_platform_halt` is a `fn() -> !`

fn main() {}
