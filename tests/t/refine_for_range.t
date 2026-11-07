// `for x in a..b` knows `x < b` in its body: `Range::next` says what it
// gives is less than the range's end, which it keeps, so the loop's frame
// keeps it too (docs/53-refined-type-arguments.md).
use alloc::list::List;

fn sum(s: &[u8]) -> u64 {
    let mut total: u64 = 0;
    for i in 0..s.len() {
        total = total + s[i] as u64;
    }
    total
}

// The body lends the list to `set`, which keeps its length: so its frame
// keeps `l.len()`, as the range's keeps its end.
fn ones(l: &mut List<u8>) ensures l.len() == old(l.len()) {
    for i in 0..l.len() {
        l.set(i, 1u8);
    }
}

// A range from a variable, nested ranges, and a const parameter's length.
fn diagonal<const N: usize>(a: &[[u8; N]; N]) -> u16 {
    let r = 0..N;
    let mut d: u16 = 0;
    for i in r {
        for j in 0..N {
            if i == j {
                d = d + a[i][j] as u16;
            }
        }
    }
    d
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let a: [u8; 4] = [1, 2, 3, 4];
    out.line(&sum(a.as_slice()))?; // expect: 10
    let mut l: List<u8> = List::filled(3, 0u8);
    ones(&mut l);
    out.line(&l[2])?; // expect: 1
    let m: [[u8; 2]; 2] = [[1, 2], [3, 4]];
    out.line(&diagonal(&m))?; // expect: 5
    let r = 3..7usize;
    out.line(&r.start())?; // expect: 3
    out.line(&r.end())?; // expect: 7
    Ok(())
}
