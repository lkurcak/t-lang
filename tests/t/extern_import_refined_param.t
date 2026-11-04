// An import may ask more of its callers than its C type does: they prove
// it, as for any function.
extern fn sleep_ms(ms: { ms: u32 | ms > 0 });

fn nap() with unsafe { sleep_ms(5) }

fn main() {
    // SAFETY: `sleep_ms` is the platform's, and only waits.
    unsafe { nap() }
// error: `sleep_ms` is a C function, which the interpreter can't call
}
