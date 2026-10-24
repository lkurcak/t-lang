// More of what closures can do.
struct Fd { n: i64 }

fn close(fd: Fd) -> i64 {
    let Fd { n } = fd;
    n
}
fn twice<effect e, F: FnMut() -> i64 with e>(f: &mut F) -> i64 with {panic, e} { f() + f() }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    // An `FnMut` closure lent by `&mut`.
    let mut n = 0;
    let mut c = || {
        n = n + 1;
        n
    };
    out.line(&twice(&mut c))?;           // expect: 3
    out.line(&twice(&mut c))?;           // expect: 7
    out.line(&n)?;                       // expect: 4
    // Through a `&mut` it captured, a closure can read, and is `Fn`.
    let mut m: i64 = 5;
    let r = &mut m;
    let get = || *r + 1;
    out.line(&(get() + get()))?;           // expect: 12
    *r = 7;
    out.line(&m)?;                       // expect: 7
    // A `move` closure changes its own copy.
    let mut k: i64 = 0;
    let mut count = move || {
        k = k + 1;
        k
    };
    out.line(&(count() + count()))?;       // expect: 3
    out.line(&k)?;                       // expect: 0
    // Borrowing what must be used up is fine: the closure may be dropped,
    // and `fd` is used up after.
    let fd = Fd { n: 4 };
    let peek = || fd.n;
    let peeked = peek();
    let closed = close(fd);
    out.line(&peeked)?;                  // expect: 4
    out.line(&closed)?;                  // expect: 4
    // In a loop, each closure borrows that time round's `i`.
    let mut total: i64 = 0;
    let mut i = 0;
    while i < 3 {
        let double = || i * 2;
        total = total + double();
        i = i + 1;
    }
    out.line(&total)?;                   // expect: 6
    Result::Ok(())
}
