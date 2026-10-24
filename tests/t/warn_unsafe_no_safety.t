// An `unsafe` block says its operations' obligations are kept; a
// `// SAFETY:` comment just before it says why.
use alloc::raw::RawBuf;

fn main() with alloc {
    let b: RawBuf<i64> = RawBuf::new(1);
    unsafe { b.free() };
}
// warning: this `unsafe` block has no `// SAFETY:` comment saying why its obligations are kept
