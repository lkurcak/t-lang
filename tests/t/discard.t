// `Discard`: which types' values may be thrown away. Built in for `int`,
// `bool`, `()`, `!`, functions and references; a box is when what it holds
// is. A struct or enum is when it says so, with `#[derive(Discard)]` or an
// impl, and `Copy` needs it.

#[allow(dead_code)]
#[derive(Discard)]
enum Option<T> { Some(T), None }

// Derived bounds come from the fields: `Callback<T>` is `Discard` for
// every `T`, and `Tagged<A, B>` only needs `A: Discard`.
#[derive(Discard)]
struct Callback<T> { f: fn(T) -> int }

#[derive(Discard)]
struct Tagged<'a, A, B> { a: Option<A>, b: &'a B }

// Recursive, through a box.
#[derive(Discard)]
enum List<T> { Cons(T, Box<List<T>>), Nil }

// Written out: every field must be `Discard`.
struct Plain { n: int }

impl Discard for Plain {}

#[derive(Discard, Copy)]
struct Point { x: int, y: int }

struct Fd { n: int }

fn close(fd: Fd) -> int {
    let Fd { n } = fd;
    n
}

fn drop_it<T: Discard>(_x: T) {}

// `Copy` gives `Discard`: `trait Copy: Discard`.
fn twice<T: Copy>(x: T) -> int {
    let _a = x;
    let _b = x;
    2
}

fn len<T: Discard>(l: &List<T>) -> int with panic {
    match l {
        List::Cons(_, rest) => 1 + len(rest),
        List::Nil => 0,
    }
}

fn takes_fd(f: Fd) -> int { close(f) }

fn main() with {alloc, panic} {
    drop_it(1);
    drop_it(true);
    drop_it(());
    drop_it(Box::new(1));
    drop_it(Plain { n: 1 });
    drop_it(Point { x: 1, y: 2 });
    let fd = Fd { n: 1 };
    // References may be thrown away, whatever they point to.
    drop_it(&fd);
    drop_it(Callback::<Fd> { f: takes_fd });
    drop_it(Tagged { a: Option::Some(1), b: &fd });
    print(close(fd));                   // expect: 1
    print(twice(Point { x: 1, y: 2 })); // expect: 2
    let l = List::Cons(1, Box::new(List::Cons(2, Box::new(List::Nil))));
    print(len(&l)); // expect: 2
}
