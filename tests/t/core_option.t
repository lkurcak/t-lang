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

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let a = half(4);
    let b = half(3);
    out.line(&a.is_some())?; // expect: true
    out.line(&b.is_none())?; // expect: true
    out.line(&a.unwrap_or(0))?;   // expect: 2
    out.line(&b.unwrap_or(7))?;   // expect: 7
    out.line(&half(10).map(|x| x * 3).unwrap_or(0))?; // expect: 15
    out.line(&half(1).map(|x| x * 3).unwrap_or(-1))?; // expect: -1
    out.line(&half(8).and_then(half).unwrap_or(0))?; // expect: 2
    out.line(&half(6).and_then(half).unwrap_or(-1))?; // expect: -1
    out.line(&half(3).or_else(|| half(4)).unwrap_or(0))?; // expect: 2
    out.line(&half(6).or_else(|| half(4)).unwrap_or(0))?; // expect: 3

    // A default made only when it's needed, so it can be linear.
    let none: Option<Fd> = Option::None;
    out.line(&close(none.unwrap_or_else(|| Fd { n: 7 })))?; // expect: 7
    out.line(&close(Option::Some(Fd { n: 8 }).unwrap_or_else(|| Fd { n: 7 })))?; // expect: 8

    // `as_ref` looks without taking; `take` takes and leaves `None`.
    let mut slot = Option::Some(Fd { n: 5 });
    let seen = match slot.as_ref() {
        Option::Some(fd) => fd.n,
        Option::None => 0,
    };
    match slot.as_mut() {
        Option::Some(fd) => fd.n = 6,
        Option::None => {}
    }
    let taken = match slot.take() {
        Option::Some(fd) => close(fd),
        Option::None => 0,
    };
    let none = slot.is_none();
    let left = match slot {
        Option::Some(fd) => close(fd),
        Option::None => 0,
    };
    out.line(&seen)?; // expect: 5
    out.line(&taken)?; // expect: 6
    out.line(&none)?; // expect: true
    out.line(&left)?; // expect: 0

    // `Option<i64>` is `Copy` and `Eq`, as `i64` is.
    let c = a;
    out.line(&(a == c))?; // expect: true
    out.line(&(a != b))?; // expect: true
    Result::Ok(())
}
