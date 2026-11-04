// The platform's two functions the runtime calls on a panic, written in
// T: each byte of the message goes to the screen, then the machine is
// turned off.
use core::num::wrapping_add;
use core::volatile::Volatile;

extern fn screen_address() -> usize;
extern fn screen_size() -> usize;
extern fn cursor_address() -> usize;
extern fn power_off(code: i32) -> !;

extern fn t_platform_write(b: u8) {
    // SAFETY: the screen's bytes and its cursor are at the addresses the
    // machine gives, and nothing else uses them; the byte is written only
    // if it's on the screen.
    unsafe {
        let cursor: &'static Volatile<usize> = Volatile::at(cursor_address());
        let at = cursor.read();
        if at < screen_size() {
            Volatile::<u8>::at(wrapping_add(screen_address(), at)).write(b);
            cursor.write(wrapping_add(at, 1));
        }
    }
}

extern fn t_platform_halt() -> ! {
    // SAFETY: turning the machine off is all `power_off` does.
    unsafe { power_off(3) }
}

fn main() with panic { core::panic::panic("the end") }
// prints: runtime error: panicked: the end
// exit: 3
