// `replace` gives back what was there and leaves what it's given, and
// `Option::take` gives back what was there and leaves `None`: so what was
// known of each value is known of it where it is after
// (docs/85-values-in-predicates.md).
use alloc::list::List;
use core::mem::replace;

fn inner(o: { o: Option<u8> | o.is_some() }) -> u8 {
    match o {
        Some(x) => x,
        None => 0,
    }
}

fn empty(o: { o: Option<u8> | o.is_none() }) -> bool { o.is_none() }

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut a: List<u8> = List::new();
    a.push(7);
    let mut b: List<u8> = List::new();
    b.push(1);
    b.push(2);
    let old = replace(&mut a, b);
    out.line(&old[0])?; // expect: 7
    out.line(&a[1])?;   // expect: 2
    let mut o: Option<u8> = Some(3);
    let t = o.take();
    out.line(&inner(t))?; // expect: 3
    out.line(&empty(o))?; // expect: true
    Ok(())
}
