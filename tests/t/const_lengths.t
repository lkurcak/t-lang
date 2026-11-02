// A const can be an array's length, or the argument of a const parameter.
// As a length it's equal only to itself (`[u8; SIZE]` isn't `[u8; 4]`),
// but what it is the checker knows from its initializer.
const SIZE: usize = 4;
const LAST: usize = SIZE - 1;

struct Buffer { data: [u8; SIZE] }

#[derive(Discard)]
struct Ring<T, const N: usize> { buf: [T; N], head: { h: usize | h < N } }

fn ring() -> Ring<u8, SIZE> { Ring { buf: [0; SIZE], head: 0 } }

fn sum<const N: usize>(a: &[u8; N]) -> u64 {
    let mut s: u64 = 0;
    for x in a.iter() {
        s = s + (*x as u64);
    }
    s
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut b = Buffer { data: [1; SIZE] };
    b.data[LAST] = 9;               // proved: LAST == 3 < 4 == SIZE
    out.line(&b.data.len())?;       // expect: 4
    out.line(&sum(&b.data))?;       // expect: 12
    let r = ring();
    out.line(&r.buf.len())?;        // expect: 4
    out.line(&r.head)?;             // expect: 0
    Result::Ok(())
}
