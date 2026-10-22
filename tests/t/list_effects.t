// Making an empty list allocates nothing, so `List::new` has no effects;
// nor do `len`, `get` or `pop`. Only what may grow the buffer allocates.
use alloc::list::List;
use alloc::raw::RawBuf;

fn empty() -> List<i64> { List::new() }

fn first(l: &List<i64>) -> Option<i64> {
    match l.get(0) {
        Option::Some(x) => Option::Some(*x),
        Option::None => Option::None,
    }
}

fn drain(l: &mut List<i64>) -> i64 {
    let mut n = 0;
    while l.pop().is_some() {
        n = checked_add(n, 1).unwrap_or(0);
    }
    n
}

fn main() with {alloc, panic} {
    let mut l = empty();
    print_bool(first(&l).is_none()); // expect: true
    l.push(5);
    l.push(6);
    print(first(&l).unwrap()); // expect: 5
    print(drain(&mut l));      // expect: 2
    l.free();

    // An empty buffer has no slots, and grows as any other.
    let mut b: RawBuf<i64> = RawBuf::empty();
    print(b.cap()); // expect: 0
    b.grow(1);
    // SAFETY: slot 0 is uninitialised, then holds the 8, which is read out
    // before the buffer is freed.
    unsafe {
        b.write(0, 8);
        print(b.read(0)); // expect: 8
        b.free();
    }
}
