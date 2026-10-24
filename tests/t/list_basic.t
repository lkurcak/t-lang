// A `List` is a dense, resizable array: `push` grows it, `pop` shrinks it,
// and `get` and `get_mut` see one value, or `None` past the end.
use alloc::boxed::Box;
use alloc::list::List;
use alloc::string::String;
use core::fmt::Write;

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = String::new();
    let mut l: List<i64> = List::new();
    out.line(&l.is_empty()); // expect: true
    out.line(&l.capacity());      // expect: 0
    let mut i = 0;
    while i < 10 {
        l.push(i * i);
        i += 1;
    }
    out.line(&l.len());                  // expect: 10
    out.line(&(l.capacity() >= 10));  // expect: true
    out.line(&(*l.get(3).unwrap()));       // expect: 9
    out.line(&l.get(10).is_none()); // expect: true
    *l.get_mut(3).unwrap() += 100;
    out.line(&(*l.get(3).unwrap()));           // expect: 109
    out.line(&l.get_mut(10).is_none()); // expect: true
    out.line(&l.pop().unwrap());             // expect: 81
    out.line(&l.pop().unwrap());             // expect: 64
    out.line(&l.len());                      // expect: 8
    l.free();

    let mut m: List<Box<i64>> = List::with_capacity(2);
    out.line(&m.capacity()); // expect: 2
    m.push(Box::new(1));
    m.push(Box::new(2));
    m.push(Box::new(3));
    out.line(&(*m.pop().unwrap())); // expect: 3
    let mut e: List<i64> = List::new();
    out.line(&e.pop().is_none()); // expect: true
    e.free();
    m.free();
    let mut stdout = os.stdout;
    let w = stdout.put(&out.as_str());
    out.free();
    w
}
