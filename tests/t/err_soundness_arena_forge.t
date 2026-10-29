// Only `Arena::new` makes an arena.
use alloc::arena::Arena;

fn main() {
    let _a: Arena<i64> = Arena {};
}
// error: `Arena` is intrinsic
