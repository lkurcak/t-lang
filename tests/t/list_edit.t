// `swap` exchanges two values, and says if both indices were in range;
// `truncate` and `clear` throw values away from the end, and keep the
// buffer.

use alloc::boxed::Box;
use alloc::list::List;
use alloc::string::String;
use core::fmt::Write;

fn show(out: &mut String, l: &List<i64>) with alloc {
    for x in l.iter() {
        out.line(x);
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = String::new();
    let mut l: List<i64> = List::new();
    for i in 0..4 {
        l.push(i);
    }
    out.line(&l.swap(0, 3)); // expect: true
    out.line(&l.swap(1, 1)); // expect: true
    out.line(&l.swap(2, 4)); // expect: false
    show(&mut out, &l);
    // expect: 3
    // expect: 1
    // expect: 2
    // expect: 0
    l.truncate(9);
    out.line(&l.len()); // expect: 4
    l.truncate(2);
    show(&mut out, &l);
    // expect: 3
    // expect: 1
    let cap = l.capacity();
    l.clear();
    out.line(&l.is_empty());        // expect: true
    out.line(&(l.capacity() == cap)); // expect: true
    l.push(7);
    show(&mut out, &l); // expect: 7
    l.discard();

    // `swap` needs no `Discard`.
    let mut b: List<Box<i64>> = List::new();
    b.push(Box::new(1));
    b.push(Box::new(2));
    b.swap(0, 1);
    out.line(&(**b.get(0).unwrap())); // expect: 2
    b.discard();
    let mut stdout = os.stdout;
    let w = stdout.put(&out.as_str());
    out.discard();
    w
}
