// A reference to a refined element given to a generic function with
// another argument of its type parameter: what the function may write
// through it is any of what it's given, so each must have what's declared
// of the element (`&mut T` is invariant in `T`).
#![allow(dead_code)]
fn set<T: Discard>(r: &mut T, v: T) { *r = v }

fn id<T>(x: T) -> T { x }

fn swapped(a: &mut [{ t: usize | t < 5 }; 2]) {
    let mut x: usize = 99;
    core::mem::swap(&mut x, &mut a[0]);
}

fn set_to(a: &mut [{ t: usize | t < 5 }; 2]) { set(&mut a[0], 99) }

fn through_id(a: &mut [{ t: usize | t < 5 }; 2]) {
    let r: &mut usize = id(&mut a[0]);
    *r = 99;
}

fn through_id_to_set(a: &mut [{ t: usize | t < 5 }; 2]) {
    let x: usize = 99;
    set(id(&mut a[0]), x);
}

fn bound_in_a_pattern(a: &mut [{ t: usize | t < 5 }; 2]) {
    let mut x: usize = 99;
    let p = Some(&mut x);
    if let Some(r) = p {
        core::mem::swap(r, &mut a[0]);
    }
}

fn main() {}
// error: can't prove `t < 5`
// error: can't prove `t < 5`
// error: can't prove `t < 5`
// error: can't prove `t < 5`
// error: can't prove `t < 5`
