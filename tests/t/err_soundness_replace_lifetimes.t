// The same for `replace`.

use core::mem::replace;

fn main() with {alloc, panic} {
    let long = Box::new(1i64);
    let mut a = &*long;
    {
        let short = Box::new(2);
        let old = replace(&mut a, &*short);
        print(*old);
    }
    print(*a);
}
// error: `short` doesn't live long enough
