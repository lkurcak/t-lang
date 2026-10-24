// A slice to change: `get_mut`, `swap`, and two halves changed at once
// with `split_at_mut`, which share no values.
use alloc::list::List;
use alloc::string::String;
use core::fmt::Write;
use core::slice::SplitMut;

fn bump(s: &mut [i64]) with panic {
    let mut i = 0;
    while i < s.len() {
        match s.get_mut(i) {
            Option::Some(x) => *x = *x + 1,
            Option::None => {},
        }
        i = i + 1;
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = String::new();
    let mut l: List<i64> = List::new();
    for i in 0..4 {
        l.push(i);
    }
    let s = l.as_mut_slice();
    bump(s);
    bump(s);
    *s.get_mut(0).unwrap() = 100;
    out.line(&s.swap(0, 3)); // expect: true
    out.line(&s.swap(1, 4)); // expect: false
    let SplitMut { head, tail } = s.split_at_mut(2).unwrap();
    *head.get_mut(1).unwrap() = *tail.get_mut(0).unwrap();
    *tail.get_mut(1).unwrap() = 7;
    for x in l.iter() {
        out.line(&(*x));
    }
    // expect: 5
    // expect: 4
    // expect: 4
    // expect: 7
    l.free();
    let mut stdout = os.stdout;
    let w = stdout.put(&out.as_str());
    out.free();
    w
}
