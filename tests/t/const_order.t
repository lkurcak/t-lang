// What the checker knows of a const's value is known wherever it's used,
// however it's named and wherever it's declared: here after its users, by
// a path, in a type, a refinement and a call.
pub fn at(a: &[u8; m::SIZE], i: { i: usize | i < m::SIZE }) -> u8 { a[i] }

fn last(a: &[u8; m::SIZE]) -> u8 { at(a, m::SIZE - 1) }

mod m {
    pub const SIZE: usize = 4;
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut a = [3u8; m::SIZE];
    a[3] = 5;
    out.line(&last(&a))?; // expect: 5
    Result::Ok(())
}
