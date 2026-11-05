// Lists compare as their slices do: `==` value by value, `<` the first
// that differ deciding, a shorter list before a longer one it starts.
use alloc::list::List;

fn list(xs: &[i64]) -> List<i64> with alloc {
    let mut l = List::new();
    for x in xs.iter() {
        l.push(*x);
    }
    l
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let a = list([1, 2, 3].as_slice());
    let b = list([1, 2, 3].as_slice());
    let c = list([1, 2].as_slice());
    let d = list([1, 3].as_slice());
    out.line(&(a == b))?;                                         // expect: true
    out.line(&(a != c))?;                                         // expect: true
    out.line(&(c < a))?;                                          // expect: true
    out.line(&(a < d))?;                                          // expect: true
    out.line(&(d > a))?;                                          // expect: true
    out.line(&(a <= b))?;                                         // expect: true
    out.line(&([2i64, 1].as_slice() < [2i64, 1, 0].as_slice()))?; // expect: true
    out.line(&([3u8].as_slice() < [2u8, 9].as_slice()))?;         // expect: false
    let less = match a.as_slice().cmp(d.as_slice()) {
        core::cmp::Ordering::Less => true,
        _ => false,
    };
    out.line(&less)?;                             // expect: true
    out.line(&a.first().copied().unwrap_or(0))?;  // expect: 1
    out.line(&a.last().copied().unwrap_or(0))?;   // expect: 3
    out.line(&a.contains(&2))?;                   // expect: true
    out.line(&a.contains(&5))?;                   // expect: false
    out.line(&[4u8, 5].as_slice().contains(&5))?; // expect: true
    let e: List<i64> = List::new();
    out.line(&e.first().is_none())?; // expect: true
    out.line(&e.last().is_none())?;  // expect: true
    Ok(())
}
