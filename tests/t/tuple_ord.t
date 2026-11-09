// Tuples compare in order, field by field, the first that differs
// deciding: `<` and the rest are `Ord`'s, which a tuple of `Ord` values
// has. `#[derive(Ord)]` gives a struct the same order, and an enum the
// order of its variants, then of their fields.
use core::cmp::max;

#[derive(Copy, Discard, Eq, Ord, Debug)]
struct Version { major: u8, minor: u8 }

#[derive(Copy, Discard, Eq, Ord, Debug)]
enum Rank { Low, Mid(u8), High(u8, bool) }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&((1u8, 'b') < (2u8, 'a')))?;            // expect: true
    out.line(&((1u8, 'b') < (1u8, 'a')))?;            // expect: false
    out.line(&((1u8, 2u8, 3u8) <= (1u8, 2u8, 3u8)))?;         // expect: true
    out.line(&f"{max((3u8, 'x'), (3u8, 'y')):?}")?;   // expect: (3, 'y')
    out.line(&f"{(3u8, 'x').cmp(&(3u8, 'x')):?}")?;   // expect: Equal
    let v = Version { major: 1, minor: 9 };
    let w = Version { major: 2, minor: 0 };
    out.line(&(v < w))?;                          // expect: true
    out.line(&f"{max(v, w):?}")?;                 // expect: Version { major: 2, minor: 0 }
    out.line(&(Rank::Low < Rank::Mid(0)))?;       // expect: true
    out.line(&(Rank::Mid(5) < Rank::Mid(4)))?;    // expect: false
    out.line(&(Rank::High(1, false) < Rank::High(1, true)))?; // expect: true
    out.line(&(Rank::High(0, false) > Rank::Mid(200)))?;     // expect: true
    // Nested, and through references.
    let a = ((1u8, 2u8), "b");
    let b = ((1u8, 2u8), "a");
    out.line(&(&a > &b))?;                        // expect: true
    Ok(())
}
