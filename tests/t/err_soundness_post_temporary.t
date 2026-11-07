// What a call ensures of a `&mut` that isn't of a place the caller knows
// is of a value nothing is known of, not of the one before the call.
use alloc::list::List;

fn grow(l: &mut List<u8>) -> u8 with alloc {
    let r = &mut *l;
    List::push(id(r), 0u8);
    let a: [u8; 1] = [7];
    a[5]
}

fn id(r: &mut List<u8>) -> &mut List<u8> { r }

fn main() {}
// error: can't prove `i < self.len()` for argument `i` of `index`
