// A `&mut` may go to another thread if what it points to may: here each
// thread gets half of an array, which no other can reach while it's lent.
use core::num::wrapping_add;

fn bump(xs: &mut [u32]) {
    for x in xs.iter_mut() {
        *x = wrapping_add(*x, 10);
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut data = [1u32, 2, 3, 4];
    if let Some(halves) = data.as_mut_slice().split_at_mut(2) {
        let core::slice::SplitMut { head, tail } = halves;
        let h = os.threads.spawn(move || bump(head));
        bump(tail);
        h.join();
    }
    let mut out = os.stdout;
    out.line(&data[0])?; // expect: 11
    out.line(&data[3])?; // expect: 14
    Ok(())
}
