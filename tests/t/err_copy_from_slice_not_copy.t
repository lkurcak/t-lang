// Only `Copy` values are copied: a list's values would then be in two
// lists, and freed twice.
use alloc::list::List;

fn copy(to: &mut [List<u8>], from: { s: &[List<u8>] | s.len() == to.len() }) {
    to.copy_from_slice(from);
}

fn main() {}
// error: `[T]::copy_from_slice` needs its `T` to implement `Copy`, and `List<u8>` doesn't
