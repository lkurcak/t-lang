// `iter` borrows a list and gives a reference to each value, front to
// back. It's `Discard`, so `for` and the adapters take it.
use alloc::list::List;
use alloc::string::String;
use core::fmt::Write;

fn sum(l: &List<i64>) -> i64 with panic {
    let mut s = 0;
    for x in l.iter() {
        s = s + *x;
    }
    s
}

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = String::new();
    let mut l: List<i64> = List::new();
    for i in 1..6 {
        l.push(i);
    }
    out.line(&sum(&l));                                               // expect: 15
    out.line(&l.iter().map(|x: &i64| *x * 10).fold(0, |a, b| a + b)); // expect: 150
    out.line(&l.iter().filter(|x: &&i64| **x % 2 == 1).count());      // expect: 3
    let mut it = l.iter();
    it.next();
    out.line(&(*it.next().unwrap())); // expect: 2
    let e: List<i64> = List::new();
    out.line(&e.iter().next().is_none()); // expect: true
    e.free();
    l.free();
    let mut stdout = os.stdout;
    let w = stdout.put(&out.as_str());
    out.free();
    w
}
