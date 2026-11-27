// A const read by index, or by field, is read where its one value is, not
// built at each use (`mono::promote`, lemma P1 of docs/102-math.md): what's
// read is the same. A variable given its value has a copy of its own,
// which can change.
const T: [(u32, u32); 4] = [(1, 2), (3, 4), (5, 6), (7, 8)];

#[derive(Copy, Discard)]
struct P { a: u32, b: [u32; 2] }

const S: P = P { a: 10, b: [20, 30] };

fn sum() -> u32 with panic {
    let mut s = 0;
    for i in 0..4usize {
        s = s + T[i].0 + T[i].1;
    }
    s + S.a + S.b[1]
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut copy = T;
    copy[0] = (100, 0);
    out.line(&sum())?;     // expect: 76
    out.line(&copy[0].0)?; // expect: 100
    out.line(&T[0].0)?;    // expect: 1
    let whole = S;
    out.line(&whole.b[0])?; // expect: 20
    Ok(())
}
