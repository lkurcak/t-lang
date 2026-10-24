// Running out of memory is part of `alloc`, not `panic`: what allocates
// says `with alloc`, and `panic` is left for bugs. None of these can panic.

use alloc::boxed::Box;
use alloc::list::List;
use alloc::raw::RawBuf;
use alloc::string::String;

fn boxed(n: i64) -> Box<i64> with alloc { Box::new(n) }

fn three() -> List<i64> with alloc {
    let mut l = List::with_capacity(1);
    l.push(1);
    l.push(2);
    l.push(3);
    l
}

fn hello() -> String with alloc {
    let mut s = String::from("hell");
    s.push('o');
    s.push_str(", wörld");
    s
}

fn buffer() -> RawBuf<i64> with alloc {
    let mut b = RawBuf::new(2);
    b.grow(3);
    b
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    out.line(&(*boxed(7)))?; // expect: 7
    let l = three();
    let three = l.len() == 3;
    l.free();
    out.line(&three)?; // expect: true
    let s = hello();
    let w = out.line(&s.as_str()); // expect: hello, wörld
    s.free();
    w?;
    let b = buffer();
    let five = b.cap() == 5;
    // SAFETY: nothing was written.
    unsafe { b.free() };
    out.line(&five) // expect: true
}
