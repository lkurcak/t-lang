// `copy_from_slice` copies one slice's values into another of the same
// length, all at once: of `Copy` values only, so nothing is moved out of
// the one copied, nor lost of the one copied into.
use alloc::list::List;

#[derive(Discard, Copy, Eq, Debug)]
struct Point { x: i32, y: i32 }

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;

    // Bytes, from a list into an array.
    let from = List::filled(4, 7u8);
    let mut to = [0u8; 4];
    to.as_mut_slice().copy_from_slice(from.as_slice());
    out.line(&f"{to[0]} {to[3]}")?; // expect: 7 7

    // Structs, between two halves of one slice.
    let mut ps = [Point { x: 1, y: 2 }, Point { x: 3, y: 4 }, Point { x: 0, y: 0 }, Point { x: 0, y: 0 }];
    if let Some((front, back)) = ps.as_mut_slice().split_at_mut(2) {
        back.copy_from_slice(front);
    }
    out.line(&f"{ps[2]:?} {ps[3]:?}")?; // expect: Point { x: 1, y: 2 } Point { x: 3, y: 4 }

    // Part of a slice, cut to the length of what's copied.
    let mut l = List::filled(6, 0u16);
    let src = [5u16, 6, 7];
    l.slice_mut(2..5).copy_from_slice(src.as_slice());
    out.line(&f"{l[1]} {l[2]} {l[4]} {l[5]}")?; // expect: 0 5 7 0

    // Nothing, into nothing.
    let none: &mut [u8] = core::slice::empty_mut();
    none.copy_from_slice(core::slice::empty());
    out.line(&none.len())?; // expect: 0

    // Refined values keep their refinement.
    let digits: [{ v: u8 | v < 10 }; 3] = [1, 2, 3];
    let mut more: [{ v: u8 | v < 10 }; 3] = [0, 0, 0];
    more.as_mut_slice().copy_from_slice(digits.as_slice());
    let sum: u8 = more[0] + more[1] + more[2];
    out.line(&sum)?; // expect: 6
    Ok(())
}
