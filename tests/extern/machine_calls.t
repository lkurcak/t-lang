// C functions the program calls, and a T function C calls back, which
// counts in a static.
use core::sync::Atomic;

extern fn add(a: i64, b: i64) -> i64;
extern fn is_even(n: u8) -> bool;
extern fn tick_times(n: u32);
extern fn power_off(code: i32) -> !;

static TICKS: Atomic<u32> = Atomic::new(0);

extern fn tick() with global {
    let _ = TICKS.fetch_add(1);
}

fn main() with global {
    // SAFETY: `add`, `is_even` and `tick_times` are the machine's, and do
    // only what they say: `tick_times` calls `tick`.
    let sum = unsafe {
        tick_times(5);
        add(40, 2)
    };
    // SAFETY: as above.
    let even = unsafe { is_even(7) };
    let code = match sum == 42 && !even && TICKS.load() == 5 {
        true => 7,
        false => 1,
    };
    // SAFETY: turning the machine off is all `power_off` does.
    unsafe { power_off(code) }
}
// exit: 7
