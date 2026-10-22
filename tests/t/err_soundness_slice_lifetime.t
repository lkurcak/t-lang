// A slice can't outlive what its values point to.
use alloc::list::List;

fn put<'a>(s: &mut [&'a int], r: &'a int) {
    match s.get_mut(0) {
        Option::Some(x) => *x = r,
        Option::None => {},
    }
}

fn main() with {alloc, panic} {
    let n = 1;
    let mut l: List<&int> = List::new();
    l.push(&n);
    {
        let m = 2;
        put(l.as_mut_slice(), &m);
    }
    print(**l.get(0).unwrap());
    l.free();
}
// error: `m` doesn't live long enough
