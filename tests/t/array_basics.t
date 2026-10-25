// `[T; N]`: exactly `N` values, inline. A literal has as many as it lists,
// a repeat `[v; n]` has `n`, and `a[i]` is proved in range from `i < N`.
fn total(s: &[i64]) -> i64 {
    let mut sum: i64 = 0;
    for x in s.iter() {
        sum = core::num::wrapping_add(sum, *x);
    }
    sum
}

#[derive(Copy, Discard)]
struct Rgb { c: [u8; 3] }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let mut a = [1, 2, 3, 4];
    a[0] = 10;
    a[3] += 30;
    out.line(&a[0])?;               // expect: 10
    out.line(&a[3])?;               // expect: 34
    out.line(&a.len())?;            // expect: 4
    out.line(&total(a.as_slice()))?; // expect: 49
    let b = a;
    a[1] = 0;
    out.line(&b[1])?;               // expect: 2
    let z: [i64; 3] = [7; 3];
    out.line(&total(z.as_slice()))?; // expect: 21
    let grid = [[0u8; 2]; 2];
    out.line(&grid[1][1])?;         // expect: 0
    let e: [bool; 0] = [];
    out.line(&e.is_empty())?;       // expect: true
    let p = Rgb { c: [255, 128, 0] };
    out.line(&p.c[1])?;             // expect: 128
    let mut i: usize = 0;
    while i < a.len() {
        a[i] = a[i] * 2;
        i = i + 1;
    }
    out.line(&a[2])?;               // expect: 6
    for x in a.iter_mut() {
        *x = *x + 1;
    }
    out.line(&a[2])?;               // expect: 7
    out.line(&(a == [21, 1, 7, 69]))?; // expect: true
    Result::Ok(())
}
