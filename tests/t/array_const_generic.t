// A const parameter `const N: usize` stands for a length the caller picks:
// it's inferred from an argument's type, or written, and it's a `usize`
// value in the body and in refinements.
fn sum<const N: usize>(a: &[i64; N]) -> i64 {
    let mut s: i64 = 0;
    let mut i: usize = 0;
    while i < N {
        s = core::num::wrapping_add(s, a[i]);
        i = i + 1;
    }
    s
}

fn at<const N: usize>(a: &[i64; N], i: { i: usize | i < N }) -> i64 { a[i] }

fn zeros<const N: usize>() -> [i64; N] { [0; N] }

/// The last `N` values pushed.
#[derive(Discard)]
struct Ring<T, const N: usize> { buf: [T; N], head: { h: usize | h < N } }

impl<T: Copy, const N: usize> Ring<T, N> {
    fn push(&mut self, v: T) {
        self.buf[self.head] = v;
        let next = self.head + 1;
        self.head = if next < N { next } else { 0 };
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&sum(&[1, 2, 3]))?;        // expect: 6
    out.line(&sum(&[5; 10]))?;          // expect: 50
    out.line(&at(&[4, 5, 6], 2))?;      // expect: 6
    let z = zeros::<3>();
    out.line(&z.len())?;                // expect: 3
    let mut r: Ring<i64, 2> = Ring { buf: [0, 0], head: 0 };
    r.push(1);
    r.push(2);
    r.push(3);
    out.line(&r.buf[0])?;               // expect: 3
    out.line(&r.buf[1])?;               // expect: 2
    Result::Ok(())
}
