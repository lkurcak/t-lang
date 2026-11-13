// A blanket impl applies to a type parameter, a projection or an `impl`
// type wherever its bounds hold there: an iterator is `IntoIterator`, so a
// `for` goes through it as it is, and an integer type is `Display`.
use core::str::Parse;

fn count<I: Iterator<Item: Discard> + Discard>(it: I) -> i64 with {panic, I::e} {
    let mut n = 0;
    for _x in it {
        n += 1;
    }
    n
}

fn first<C: IntoIterator>(c: C) -> Option<i64>
where
    C::IntoIter: Iterator<Item = i64, e = {}> + Discard,
{
    for x in c.into_iter() {
        return Some(x);
    }
    None
}

fn same<I: Iterator>(it: I) -> <I as IntoIterator>::IntoIter { it }

fn shown<T: Int>(out: &mut std::io::Stdout, x: T) -> Result<(), std::io::Error> {
    out.line(&f"{x} {x:?}")
}

fn read<T: Int>(s: &str) -> Result<T, <T as Parse>::Error> { s.parse::<T>() }

fn evens() -> impl Iterator<Item = i64, e = {}> + Discard { (0..6i64).filter(|x: &i64| *x % 2 == 0) }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&count(0..5i64))?; // expect: 5
    out.line(&count(evens()))?; // expect: 3
    out.line(&first(evens()).unwrap_or(-1))?; // expect: 0
    out.line(&count(same(1..3i64)))?; // expect: 2
    shown(&mut out, 3u8)?; // expect: 3 3
    match read::<u8>("42") {
        Ok(n) => out.line(&n)?, // expect: 42
        Err(_) => {},
    }
    let mut seen = 0;
    for x in evens() {
        seen += x;
    }
    out.line(&seen) // expect: 6
}
