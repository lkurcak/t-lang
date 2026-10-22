// `Option` comes from `core`, through the prelude: no `use` needed. Its
// methods that would throw a value away ask for `Discard` where they do,
// so an `Option` of something linear works with the rest.

struct Fd { n: i64 }

fn close(fd: Fd) -> i64 {
    let Fd { n } = fd;
    n
}

fn half(n: i64) -> Option<i64> {
    if n % 2 == 0 { Option::Some(n / 2) } else { Option::None }
}

fn main() with panic {
    let a = half(4);
    let b = half(3);
    print_bool(a.is_some()); // expect: true
    print_bool(b.is_none()); // expect: true
    print(a.unwrap_or(0));   // expect: 2
    print(b.unwrap_or(7));   // expect: 7
    print(half(10).map(|x| x * 3).unwrap_or(0)); // expect: 15
    print(half(1).map(|x| x * 3).unwrap_or(-1)); // expect: -1
    print(half(8).and_then(half).unwrap_or(0)); // expect: 2
    print(half(6).and_then(half).unwrap_or(-1)); // expect: -1
    print(half(3).or_else(|| half(4)).unwrap_or(0)); // expect: 2
    print(half(6).or_else(|| half(4)).unwrap_or(0)); // expect: 3

    // A default made only when it's needed, so it can be linear.
    let none: Option<Fd> = Option::None;
    print(close(none.unwrap_or_else(|| Fd { n: 7 }))); // expect: 7
    print(close(Option::Some(Fd { n: 8 }).unwrap_or_else(|| Fd { n: 7 }))); // expect: 8

    // `as_ref` looks without taking; `take` takes and leaves `None`.
    let mut slot = Option::Some(Fd { n: 5 });
    match slot.as_ref() {
        Option::Some(fd) => print(fd.n), // expect: 5
        Option::None => {}
    }
    match slot.as_mut() {
        Option::Some(fd) => fd.n = 6,
        Option::None => {}
    }
    match slot.take() {
        Option::Some(fd) => print(close(fd)), // expect: 6
        Option::None => {}
    }
    print_bool(slot.is_none()); // expect: true
    match slot {
        Option::Some(fd) => print(close(fd)),
        Option::None => print(0i64), // expect: 0
    }

    // `Option<i64>` is `Copy` and `Eq`, as `i64` is.
    let c = a;
    print_bool(a == c); // expect: true
    print_bool(a != b); // expect: true
}
