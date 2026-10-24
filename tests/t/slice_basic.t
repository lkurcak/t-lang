// A list's values, as a slice: `len`, `get`, `first`, `last`, and taking it
// apart with `split_at`.
use alloc::list::List;
use alloc::string::String;
use core::fmt::Write;
use core::slice::Split;

fn sum(s: &[i64]) -> i64 with panic {
    let mut total = 0;
    let mut i = 0;
    while i < s.len() {
        total = total + *s.get(i).unwrap();
        i = i + 1;
    }
    total
}

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = String::new();
    let mut l: List<i64> = List::new();
    for i in 1..5 {
        l.push(i * 10);
    }
    let s = l.as_slice();
    out.line(&s.len());                  // expect: 4
    out.line(&s.is_empty());        // expect: false
    out.line(&(*s.first().unwrap()));      // expect: 10
    out.line(&(*s.last().unwrap()));       // expect: 40
    out.line(&s.get(4).is_none());  // expect: true
    out.line(&sum(s));                   // expect: 100
    let Split { head, tail } = s.split_at(1).unwrap();
    out.line(&head.len());                        // expect: 1
    out.line(&sum(tail));                         // expect: 90
    out.line(&s.split_at(5).is_none());      // expect: true
    out.line(&s.split_at(4).unwrap().tail.len()); // expect: 0
    out.line(&sum(s));                            // expect: 100
    l.free();
    let mut stdout = os.stdout;
    let w = stdout.put(&out.as_str());
    out.free();
    w
}
