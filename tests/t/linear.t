// Linear types: a value whose type isn't `Discard` must be used up, on
// every path that goes on, rather than thrown away. Here that's `Fd`, a
// stand-in for a file descriptor, which only `close` uses up.

struct Fd { n: i64 }

fn open(n: i64) -> Fd { Fd { n } }

// Taking it apart, by value, uses it up.
fn close(fd: Fd) -> i64 {
    let Fd { n } = fd;
    n
}

fn peek(fd: &Fd) -> i64 { fd.n }

#[allow(dead_code)]
#[derive(Discard)]
enum Option<T> { Some(T), None }

struct Pair { a: Fd, b: Fd }

#[allow(dead_code)]
enum Either { Left(Fd), Right(Fd) }

fn die() -> ! {
    loop {}
}

// Every path uses it up, one way or another.
fn branch(c: bool, fd: Fd) -> i64 with panic {
    if c { close(fd) } else { 10 + close(fd) }
}

// A path that never goes on is exempt: it never comes to dropping `fd`.
fn checked(n: i64, fd: Fd) -> i64 {
    if n < 0 {
        die();
    }
    close(fd)
}

// Giving it back is using it up, too.
fn pass(fd: Fd) -> Fd { fd }

fn main() with {panic, alloc} {
    let fd = open(1);
    print(peek(&fd));              // expect: 1
    print(close(pass(fd)));        // expect: 1
    print(branch(true, open(2)));  // expect: 2
    print(branch(false, open(2))); // expect: 12
    print(checked(3, open(3)));    // expect: 3

    // A pattern that takes it apart uses it up; matching an enum by
    // value uses it up in every arm, `None` too.
    let o = Option::Some(open(4));
    match o {
        Option::Some(fd) => print(close(fd)), // expect: 4
        Option::None => {},
    }
    let e = Either::Right(open(5));
    match e {
        Either::Left(fd) | Either::Right(fd) => print(close(fd)), // expect: 5
    }
    let nested = Option::Some(Pair { a: open(6), b: open(7) });
    match nested {
        Option::Some(Pair { a, b }) => print(close(a) + close(b)), // expect: 13
        Option::None => {},
    }

    // Moving every part out uses it up, as taking it apart does.
    let p = Pair { a: open(8), b: open(9) };
    let a = p.a;
    print(close(a) + close(p.b)); // expect: 17

    // A part that's gone can be given a new value.
    let mut q = Pair { a: open(1), b: open(2) };
    let old = q.a;
    q.a = open(10);
    print(close(old) + close(q.a) + close(q.b)); // expect: 13

    // A box of one is used up through `*`.
    let b = Box::new(open(11));
    print(close(*b)); // expect: 11

    // Round a loop: given a new value only once the old one is gone.
    let mut fd = open(0);
    let mut i = 0;
    while i < 3 {
        print(close(fd)); // expect: 0
                          // expect: 1
                          // expect: 2
        i += 1;
        fd = open(i);
    }
    print(close(fd)); // expect: 3

    // Matching through a reference only looks.
    let fd = open(12);
    match &fd {
        Fd { n } => print(*n), // expect: 12
    }
    print(close(fd)); // expect: 12
}
