// A linked list on the heap: built by moving nodes into boxes, read and
// changed through references, and freed when it goes away.

#[derive(Discard)]
enum List { Nil, Cons(int, Box<List>) }

fn push(l: List, x: int) -> List with {alloc, panic} { List::Cons(x, Box::new(l)) }

fn sum(l: &List) -> int with panic {
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

fn main() with {alloc, panic} {
    let mut l = List::Nil;
    let mut i = 1;
    while i <= 4 {
        l = push(l, i);
        i += 1;
    }
    print(sum(&l));
    double(&mut l);
    print(sum(&l));
}
