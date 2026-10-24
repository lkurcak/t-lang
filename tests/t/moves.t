// Moves: a value that isn't `Copy` has one owner. Using it moves it to the
// new one, after which the old one can't be used, until it's given a new
// value.

use alloc::boxed::Box;

#[derive(Discard, Copy)]
struct Point { x: i64, y: i64 }

#[derive(Discard)]
struct Owner { name: i64, data: Box<i64> }

#[allow(dead_code)]
#[derive(Discard)]
enum Option<T> { Some(T), None }

fn take(o: Owner) -> i64 with panic { o.name + *o.data }

fn main() with {panic, alloc} {
    let p = Point { x: 1, y: 2 };
    let q = p;                  // copied
    print(p.x + q.y);           // expect: 3

    let o = Owner { name: 10, data: Box::new(5) };
    let d = o.data;             // moves only the field out
    print(o.name);              // expect: 10
    print(*d);                  // expect: 5

    let mut b = Box::new(1i64);
    let c = b;                  // moves `b`...
    b = Box::new(2);            // ...which can then get a new value
    print(*b + *c);             // expect: 3

    let o2 = Owner { name: 1, data: Box::new(2) };
    print(take(o2));            // expect: 3

    // A `match` moves out of what it matches only what it binds.
    let opt = Option::Some(Box::new(4i64));
    match opt {
        Option::Some(inner) => print(*inner), // expect: 4
        Option::None => {}
    }
    let pair = Option::Some(Point { x: 7, y: 8 });
    match pair {
        Option::Some(pt) => print(pt.y), // expect: 8
        Option::None => {}
    }
    match pair {
        Option::Some(Point { x, .. }) => print(x), // expect: 7
        Option::None => {}
    }
}
