// An alias can take const parameters, and an array type's length can
// be one.
type Buf<const N: usize> = [u8; N];
type Two<T> = [T; 2];

fn total<const N: usize>(b: &Buf<N>) -> u64 {
    let mut sum = 0u64;
    for x in b.iter() {
        sum = sum + *x as u64;
    }
    sum
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let b: Buf<3> = [1, 2, 3];
    out.line(&total(&b))?; // expect: 6
    let t: Two<bool> = [true, false];
    out.line(&t[0])?; // expect: true
    Result::Ok(())
}
