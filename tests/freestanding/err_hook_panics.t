// The runtime writes a panic's message with it: a panic in it would never
// end.
extern fn t_platform_write(b: u8) { // error: so it can't have `panic`
    let _ = 255 / b;
}

fn main() {}
