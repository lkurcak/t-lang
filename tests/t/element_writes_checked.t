// Writes through references to refined elements, kept, passed on and
// given to generic functions, are checked against what's declared of the
// elements, and those that keep it are fine (docs/63).
use alloc::list::List;

fn set<T: Discard>(r: &mut T, v: T) { *r = v }

fn small(r: &mut { t: usize | t < 5 }) { *r = 4 }

fn apply<T, F: FnMut(&mut T) + Discard>(r: &mut T, mut f: F) { f(r) }

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = os.stdout;
    let mut l: List<{ t: usize | t < 5 }> = List::new();
    l.push(1);
    l.push(2);
    let mut arr: [{ t: usize | t < 5 }; 2] = [0, 1];
    let mut pairs: List<(u8, { v: usize | v < 5 })> = List::new();
    pairs.push((0, 1));
    {
        let r = &mut l[0];
        *r = 3;
    }
    let q: &mut { t: usize | t < 5 } = &mut arr[1];
    *q = 2;
    pairs[0].1 = 4;
    core::mem::swap(&mut arr[0], &mut l[1]);
    set(&mut arr[1], 3);
    small(&mut l[0]);
    for x in l.iter_mut().rev() {
        *x = 4 - *x;
    }
    for (k, x) in arr.iter_mut().enumerate() {
        if k < 4 {
            *x = k;
        }
    }
    apply(&mut arr[0], |x| *x = 1);
    apply(&mut l[1], |x| *x = 2);
    let g: [u8; 5] = [10, 11, 12, 13, 14];
    out.line(&f"{g[l[0]]} {g[l[1]]} {g[arr[0]]} {g[arr[1]]} {g[pairs[0].1]}")?; // expect: 10 12 11 11 14
    Ok(())
}
