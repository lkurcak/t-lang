// An `extern fn` in a module is C's by its own name, not its path: the
// C library's `putchar`, imported in a module, and an export in another.
mod c {
    pub extern fn putchar(c: i32) -> i32;
}

mod events {
    pub extern fn on_event(x: i32) -> i32 { x + 1 }
}

fn main() {
    // SAFETY: `putchar` writes a byte to `stdout`.
    unsafe {
        let _ = c::putchar(104);
        let _ = c::putchar(events::on_event(104));
        let _ = c::putchar(10);
    }
}
// prints: hi
