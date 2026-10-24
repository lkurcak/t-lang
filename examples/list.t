//! A linked list on the heap: built by moving nodes into boxes, read and
//! changed through references, and freed when it goes away.

use alloc::boxed::Box;

#[derive(Discard)]
enum List { Nil, Cons(i64, Box<List>) }

fn push(l: List, x: i64) -> List with alloc { List::Cons(x, Box::new(l)) }

fn sum(l: &List) -> i64 with panic {
    match l {
        List::Nil => 0,
        List::Cons(x, rest) => *x + sum(rest),
    }
}

fn double(l: &mut List) with panic {
    match l {
        List::Nil => {},
        List::Cons(x, rest) => {
            *x *= 2;
            double(rest);
        },
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = os.stdout;
    let mut l = List::Nil;
    let mut i = 1;
    while i <= 4 {
        l = push(l, i);
        i += 1;
    }
    out.line(&sum(&l))?; // expect: 10
    double(&mut l);
    out.line(&sum(&l))?; // expect: 20
    Result::Ok(())
}
