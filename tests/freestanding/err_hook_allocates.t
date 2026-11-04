// Nor can it need memory, which may be what ran out.
extern fn t_platform_write(b: u8) { // error: so it can't have `alloc`
    let _ = alloc::boxed::Box::new(b);
}

fn main() {}
