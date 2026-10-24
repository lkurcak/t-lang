// A list of references can't outlive what they point to.
use alloc::list::List;

fn main() with {alloc, panic} {
    let mut l: List<&i64> = List::new();
    {
        let n = 1;
        l.push(&n);
    }
    let _v = **l.get(0).unwrap();
    l.free();
}
// error: `n` doesn't live long enough: it goes out of scope here while still borrowed
