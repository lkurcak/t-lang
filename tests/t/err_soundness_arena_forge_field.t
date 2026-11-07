// Only `Arena::new` makes an arena: its field is private to
// `alloc::arena`, and so are the slots it keeps its values in.
use alloc::arena::Arena;
use core::cell::Unsafe;

fn main() {
    let _a: Arena<i64> = Arena { slots: Unsafe::new(alloc::arena::Slots::new()) };
    // error: private
}
