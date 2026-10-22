// A list of linear values can't be dropped, nor `free`d, which would drop
// them: `free_with` hands each to a function, front to back, and frees the
// rest. A list of lists is used up the same way.
use alloc::list::List;

struct Fd { n: i64 }

fn close(fd: Fd) {
    let Fd { n } = fd;
    print(n);
}

fn main() with {alloc, panic} {
    let mut l: List<Fd> = List::new();
    l.push(Fd { n: 1 });
    l.push(Fd { n: 2 });
    l.push(Fd { n: 3 });
    close(l.pop().unwrap()); // expect: 3
    l.free_with(close);
    // expect: 1
    // expect: 2

    let mut ll: List<List<i64>> = List::new();
    let mut a: List<i64> = List::new();
    a.push(4);
    ll.push(a);
    ll.push(List::new());
    ll.free_with(|x: List<i64>| {
        print(x.len());
        x.free()
    });
    // expect: 1
    // expect: 0
}
