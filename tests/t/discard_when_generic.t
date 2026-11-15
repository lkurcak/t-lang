// Generic code drops what it knows is empty, whatever the type of its
// values: at `i64`, which is `Discard`, the list's own `discard` runs; at
// `Fd`, the one for empty lists (docs/80).
use alloc::list::List;

struct Fd { n: i64 }

fn each<effect e, T, F: FnMut(T) with e + Discard>(l: List<T>, mut f: F) with e {
    for x in l {
        f(x);
    }
}

fn emptied<effect e, T, F: FnMut(T) with e + Discard>(mut l: List<T>, mut f: F) with e {
    while let Some(x) = l.pop() {
        f(x);
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = os.stdout;
    let mut a: List<i64> = List::new();
    a.push(1);
    a.push(2);
    let mut total = 0;
    each(a, |x| total = total + x);
    out.line(&total)?; // expect: 3
    let mut b: List<Fd> = List::new();
    b.push(Fd { n: 4 });
    b.push(Fd { n: 5 });
    let mut sum = 0;
    each(b, |fd| {
        let Fd { n } = fd;
        sum = sum + n;
    });
    out.line(&sum)?; // expect: 9
    let mut c: List<Fd> = List::new();
    c.push(Fd { n: 6 });
    let mut last = 0;
    emptied(c, |fd| {
        let Fd { n } = fd;
        last = n;
    });
    out.line(&last)?; // expect: 6
    Ok(())
}
