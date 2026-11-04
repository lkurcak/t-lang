// A handle put in an arena isn't lost: an arena of what must be used up
// can't be dropped either.
use alloc::arena::Arena;
use std::thread::JoinHandle;

fn main(os: std::Os) with alloc {
    let data = [1u32];
    let arena: Arena<JoinHandle<'_, u32>> = Arena::new();
    let _ = arena.alloc(os.threads.spawn(|| data[0]));
}
// error: `arena` is never used up, and its type, `Arena<JoinHandle<u32>>`, isn't `Discard`
