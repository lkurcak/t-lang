// An arena hands out what it holds through a `&`, so it isn't `Sync`.
use alloc::arena::Arena;

fn sync<T: Sync>(_t: &T) {}

fn main() with {alloc, panic} {
    let a: Arena<u8> = Arena::new();
    sync(&a);
// error: `sync` needs its `T` to implement `Sync`, and `Arena<u8>` doesn't
}
