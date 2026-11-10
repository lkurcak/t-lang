// A reference to a refined element, inside an enum or a tuple given to a
// function: what's behind it is declared as the element is.
#![allow(dead_code)]
fn bad_option(o: Option<&mut usize>) {
    if let Some(r) = o {
        *r = 99;
    }
}

fn bad_tuple(t: (&mut usize, u8)) { *t.0 = 99 }

fn some(a: &mut [{ t: usize | t < 5 }; 2]) { bad_option(Some(&mut a[0])) }

fn kept(a: &mut [{ t: usize | t < 5 }; 2]) {
    let p = Some(&mut a[0]);
    bad_option(p);
}

fn pair(a: &mut [{ t: usize | t < 5 }; 2]) { bad_tuple((&mut a[0], 1)) }

fn main() {}
// error: can't prove `t < 5`
// error: can't prove `t < 5`
// error: can't prove `t < 5`
