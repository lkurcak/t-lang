// `replace` writes the value it's given behind its `&mut`: so that must
// have the place's refinement.
fn main() {
    let mut a: { v: u32 | v > 0 } = 1;
    let z: u32 = 0;
    let _ = core::mem::replace(&mut a, z);
    let _ = a;
}
// error: can't prove `v > 0` for argument `src` of `replace`
// error: can't prove `v > 0` for `a` after the call to `replace`
