// Each `extern fn` is C's by its own name, wherever it's declared: the
// platform's two functions, imports, and an export C calls back, all in
// modules.
mod machine {
    pub extern fn screen_address() -> usize;
    pub extern fn screen_size() -> usize;
    pub extern fn cursor_address() -> usize;
    pub extern fn power_off(code: i32) -> !;
    pub extern fn tick_times(n: u32);
}

mod platform {
    use core::num::wrapping_add;
    use core::volatile::Volatile;
    use super::machine::{cursor_address, power_off, screen_address, screen_size};

    extern fn t_platform_write(b: u8) {
        // SAFETY: the screen's bytes and its cursor are at the addresses
        // the machine gives, and nothing else uses them; the byte is
        // written only if it's on the screen.
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
}

mod counter {
    use core::sync::Atomic;

    pub static TICKS: Atomic<u32> = Atomic::new(0);

    extern fn tick() with global {
        let _ = TICKS.fetch_add(1);
    }
}

fn main() with {global, panic} {
    // SAFETY: `tick_times` is the machine's, and only calls `tick`.
    unsafe { machine::tick_times(5) };
    if counter::TICKS.load() == 5 { core::panic::panic("ticked 5 times") }
}
// prints: runtime error: panicked: ticked 5 times
// exit: 3
