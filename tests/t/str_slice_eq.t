// Slices compare value by value, when their values do.
use alloc::list::List;

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut a: List<i64> = List::new();
    a.push(1);
    a.push(2);
    let mut b: List<i64> = List::new();
    b.push(1);
    let shorter = a.as_slice() == b.as_slice();
    b.push(2);
    let same = a.as_slice() == b.as_slice();
    a.discard();
    b.discard();
    out.line(&shorter)?; // expect: false
    out.line(&same) // expect: true
}
