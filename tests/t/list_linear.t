// A list of linear values can't be dropped, nor `free`d, which would drop
// them: `free_with` hands each to a function, front to back, and frees the
// rest. A list of lists is used up the same way.
use alloc::list::List;
use alloc::string::String;
use core::fmt::Write;

struct Fd { n: i64 }

fn close(fd: Fd) -> i64 {
    let Fd { n } = fd;
    n
}

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = String::new();
    let mut l: List<Fd> = List::new();
    l.push(Fd { n: 1 });
    l.push(Fd { n: 2 });
    l.push(Fd { n: 3 });
    out.line(&close(l.pop().unwrap())); // expect: 3
    l.free_with(|fd: Fd| {
        out.line(&close(fd));
    });
    // expect: 1
    // expect: 2

    let mut ll: List<List<i64>> = List::new();
    let mut a: List<i64> = List::new();
    a.push(4);
    ll.push(a);
    ll.push(List::new());
    ll.free_with(|x: List<i64>| {
        out.line(&x.len());
        x.free()
    });
    // expect: 1
    // expect: 0
    let mut stdout = os.stdout;
    let w = stdout.put(&out.as_str());
    out.free();
    w
}
