// `swap` makes both places hold a reference that lives as long as the
// other did: `a` can't end up pointing into `short`.

use core::mem::swap;

fn main() with {alloc, panic} {
    let long = Box::new(1);
    let mut a = &*long;
    {
        let short = Box::new(2);
        let mut b = &*short;
        swap(&mut a, &mut b);
        print(*b);
    }
    print(*a);
}
// error: `short` doesn't live long enough
