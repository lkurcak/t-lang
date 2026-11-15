// A closure that owns an empty list of linear values isn't `Discard`: what
// it owns is checked by type, and nothing about its value is known there.
use alloc::list::List;

struct Fd { n: i64 }

fn main() with alloc {
    let l: List<Fd> = List::new();
    let c = move || l.len();
}
// warning: unused variable `c`: use it up
// error: `c` is never used up
