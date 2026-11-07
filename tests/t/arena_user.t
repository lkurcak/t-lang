// A pool written in the program itself, over `Unsafe`: a list of boxes,
// lending each value as `&mut` while the pool is borrowed. (Unlike alloc's
// `Arena`, its values can't point to each other: dropping the list runs
// `List`'s own `discard`, which the drop check can't see into.)
use alloc::boxed::Box;
use alloc::list::List;
use core::cell::Unsafe;

#[derive(Discard where T: Discard)]
struct Pool<T> { items: Unsafe<List<Box<T>>> }

impl<T> Pool<T> {
    fn new() -> Pool<T> { Pool { items: Unsafe::new(List::new()) } }

    fn put(&self, v: T) -> &mut T with alloc {
        // SAFETY: the list's `&mut` lives only in here; a loan of a value
        // is into its box, which moving the box doesn't move.
        let items = unsafe { self.items.as_mut() };
        items.push(Box::new(v));
        let i = items.len() - 1;
        &mut *items[i]
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let p = Pool::new();
    let a = p.put(1i64);
    let b = p.put(2i64);
    *a = *a + 10;
    let mut out = os.stdout;
    out.line(&(*a + *b))?; // expect: 13
    Ok(())
}
