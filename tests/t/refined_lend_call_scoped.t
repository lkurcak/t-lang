// A place whose type has a refinement can be lent `&mut` straight to a call
// that can't keep the reference, behind a `&mut` parameter or as a refined
// `let`: after the call, the refinement is proved again, of what the call
// left there, which its `ensures` says (docs/61-proof-search.md).
use alloc::list::List;

fn mark(n: usize, l: &mut { l: List<u8> | l.len() == n }, v: { v: usize | v < n }) {
    l.set(v, 1);
    let _ = l.replace(v, 2);
    l[v] = core::num::wrapping_add(l[v], 1);
    if n >= 2 {
        l.swap(0, 1);
    }
}

fn marked(n: usize, v: { v: usize | v < n }) -> { l: List<u8> | l.len() == n } with alloc {
    let mut l: { l: List<u8> | l.len() == n } = List::filled(n, 0);
    l.set(v, 1);
    mark(n, &mut l, v);
    l[v] = 9;
    l
}

// A `&mut` to a list whose type says nothing of its length can be given
// where one is needed, and pushed to after: what the call needed of it
// isn't what's declared of it.
fn mark_then_push(l: &mut List<u8>) with alloc {
    let n = l.len();
    if n > 0 {
        mark(n, l, 0);
    }
    l.push(7);
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let l = marked(3, 2);
    for x in l.iter() {
        out.line(x)?;
    }
    // expect: 0
    // expect: 0
    // expect: 9
    let mut k = List::filled(2, 5u8);
    mark(2, &mut k, 0);
    for x in k.iter() {
        out.line(x)?;
    }
    // expect: 5
    // expect: 3
    mark_then_push(&mut k);
    out.line(&k.len())?; // expect: 3
    Ok(())
}
