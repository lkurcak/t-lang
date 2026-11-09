// A place whose type has refinements only in type arguments can be
// borrowed `&mut` into a variable: the reference's type has them, so what's
// written through it must have them too (docs/60-refined-type-arguments.md).
use alloc::list::List;
use core::num::wrapping_add;

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let a: [u64; 4] = [1, 10, 100, 1000];
    let mut q: List<{ t: usize | t < 4 }> = List::new();
    let r = &mut q;
    r.push(1);
    r.push(3);
    if r.len() > 0 {
        r[0] = 2;
    }
    let mut s: u64 = 0;
    for i in q.iter() {
        s = wrapping_add(s, a[*i]);
    }
    out.line(&s)?; // expect: 1100
    Ok(())
}
