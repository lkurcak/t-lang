// `[v; n]` makes `n` copies of `v`, so `v` must be `Copy`.
use alloc::list::List;

fn main() with alloc {
    let l: List<i64> = List::new();
    let _a = [l; 2];
}
// error: `[v; n]` copies `v`, but `List<i64>` isn't `Copy`
