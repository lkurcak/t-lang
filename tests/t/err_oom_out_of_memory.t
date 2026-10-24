// `out_of_memory` aborts, as allocating does when there's no memory left:
// what a collection calls when it can't count what it holds.
use alloc::raw::out_of_memory;

fn give_up(n: i64) -> i64 with alloc {
    if n == 0 {
        out_of_memory();
    }
    n
}

fn main() with alloc {
    print(give_up(1)); // expect: 1
    give_up(0);
}
// error: runtime error: out of memory
