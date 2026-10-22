// An empty slice points at nothing, so one can be made from nowhere, of
// any lifetime, even to change.
use core::slice;

fn main() {
    let s: &[i64] = slice::empty();
    print(s.len());                        // expect: 0
    print_bool(s.first().is_none());       // expect: true
    print_bool(s.iter().next().is_none()); // expect: true
    let m: &mut [bool] = slice::empty_mut();
    print_bool(m.get_mut(0).is_none()); // expect: true
    print_bool(m.swap(0, 0));           // expect: false
}
