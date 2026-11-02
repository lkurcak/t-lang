// What it calls can be generic over effects, so long as at this call they
// come to at most `panic`.
fn call<effect e>(f: impl Fn() -> u8 with e) -> u8 with e { f() }

const N: u8 = call(|| 1);

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&N)?; // expect: 1
    Result::Ok(())
}
