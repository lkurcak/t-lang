// A program that writes the platform's functions is for the freestanding
// target: on the hosted one, the platform is the C library's, so it
// doesn't run there. It checks the same on both, and its tests, each a
// hosted program of its own, call them as the T functions they are.
use core::sync::Atomic;

static WRITTEN: Atomic<u32> = Atomic::new(0);

extern fn t_platform_write(b: u8) with global {
    let _ = WRITTEN.fetch_add(b as u32);
}
// error: this program writes the platform's `t_platform_write`, which on the hosted target is the C library's

extern fn t_platform_halt() -> ! {
    loop {}
}

fn main() {}

#[test]
fn writes_add_up() with {global, panic} {
    t_platform_write(2);
    t_platform_write(3);
    assert_eq(&WRITTEN.load(), &5);
}
// test: writes_add_up ... ok
