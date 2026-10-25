// What a struct's type needs of its arguments holds of one in an array
// a function is given, so it may count on it: `Slot<'a, &'b i64>` needs
// `&'b i64: 'a`, so `'b` outlives `'a`.
#[derive(Copy, Discard)]
struct Slot<'a, T: 'a> { r: &'a T }

fn get<'a, 'b>(s: &[Slot<'a, &'b i64>; 1]) -> &'a i64 { *s[0].r }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let n: i64 = 4;
    let r = &n;
    out.line(get(&[Slot { r: &r }]))?; // expect: 4
    Result::Ok(())
}
