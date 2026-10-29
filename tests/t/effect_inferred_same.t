// Where a type must equal another, its effects must too: `g`'s type, in
// `g` itself, is only known once its group's effects are. Here they're
// what the annotation says, so it holds.
fn g(n: u8) -> i64 {
    let _keep: Option<fn(u8) -> i64 with panic> = Option::Some(g);
    if n == 0 { 1 } else if n == 9 { panic("boom") } else { g(n - 1) }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&g(3))?; // expect: 1
    Result::Ok(())
}
