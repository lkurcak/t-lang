// A type can hold itself through a box: a linked list.

enum List { Nil, Cons(int, Box<List>) }

fn push(l: List, x: int) -> List { List::Cons(x, Box::new(l)) }

// `l` is a reference, so `match` looks through it, and binds `x` and
// `rest` by reference: `rest` is a `&Box<List>`, which is used as the
// `&List` `sum` wants.
fn sum(l: &List) -> int {
    match l {
        List::Nil => 0,
        List::Cons(x, rest) => *x + sum(rest),
    }
}

fn len(l: &List) -> int {
    let mut n = 0;
    let mut at = l;
    let mut more = true;
    while more {
        match at {
            List::Nil => more = false,
            List::Cons(_, rest) => {
                n += 1;
                at = rest;
            }
        }
    }
    n
}

// Adds one to every element, in place.
fn bump(l: &mut List) {
    match l {
        List::Nil => {}
        List::Cons(x, rest) => {
            *x += 1;
            bump(rest);
        }
    }
}

// Reverses the list by moving each node over, freeing nothing.
fn reverse(l: List) -> List {
    let mut done = List::Nil;
    let mut todo = l;
    let mut more = true;
    while more {
        match todo {
            List::Nil => more = false,
            List::Cons(x, rest) => {
                done = List::Cons(x, Box::new(done));
                todo = *rest;
            }
        }
    }
    done
}

fn first(l: &List) -> int {
    match l {
        List::Nil => 0,
        List::Cons(x, _) => *x,
    }
}

fn main() {
    let mut l = push(push(push(List::Nil, 1), 2), 3);
    print(sum(&l));             // expect: 6
    print(len(&l));             // expect: 3
    bump(&mut l);
    print(sum(&l));             // expect: 9
    print(first(&l));           // expect: 4
    let r = reverse(l);
    print(first(&r));           // expect: 2
    print(len(&r));             // expect: 3
}
