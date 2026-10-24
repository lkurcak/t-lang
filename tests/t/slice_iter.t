// `iter` and `iter_mut` over slices are safe code: `iter_mut` takes the
// rest of the slice apart with `split_at_mut`, one value at a time.
use alloc::list::List;
use alloc::string::String;
use core::fmt::Write;

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = String::new();
    let mut l: List<i64> = List::new();
    for i in 1..5 {
        l.push(i);
    }
    for x in l.iter_mut() {
        *x = *x * *x;
    }
    for x in l.as_mut_slice().iter_mut() {
        *x = *x + 1;
    }
    out.line(&l.iter().fold(0, |a, x: &i64| a + *x)); // expect: 34
    let mut it = l.iter_mut();
    let a = it.next().unwrap();
    let b = it.next().unwrap();
    *a = *b;
    out.line(&(*l.get(0).unwrap())); // expect: 5
    let mut e = l.as_slice().iter();
    e.next();
    out.line(&e.count()); // expect: 3
    l.free();
    let mut stdout = os.stdout;
    let w = stdout.put(&out.as_str());
    out.free();
    w
}
