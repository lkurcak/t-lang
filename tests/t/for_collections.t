// `for` goes through what implements `IntoIterator`: references to lists,
// slices and arrays give references to their values, a list by value gives
// its values, and an array of copies its copies.
use alloc::list::List;
use core::num::wrapping_add;

fn total<C: IntoIterator>(c: C) -> i64 with C::IntoIter::e
where
    C::IntoIter: Iterator<Item = i64> + Discard,
{
    let mut n = 0i64;
    for x in c {
        n = wrapping_add(n, x);
    }
    n
}

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = os.stdout;
    let mut l: List<i64> = List::new();
    l.push(1);
    l.push(2);
    l.push(3);
    let mut sum = 0;
    for x in &l {
        sum += *x;
    }
    out.line(&sum)?; // expect: 6
    for x in &mut l {
        *x *= 10;
    }
    out.line(&l[2])?; // expect: 30
    let a = [4i64, 5, 6];
    for x in &a {
        sum += *x;
    }
    out.line(&sum)?; // expect: 21
    for x in a {
        sum += x;
    }
    out.line(&sum)?; // expect: 36
    for x in l.as_slice() {
        sum += *x;
    }
    out.line(&sum)?; // expect: 96
    out.line(&total(0..4i64))?; // expect: 6
    out.line(&total(a))?; // expect: 15
    out.line(&total(l)) // expect: 60
}
