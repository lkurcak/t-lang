// A function type gains effects only as a whole value, not inside another
// type: there's no subtyping, so `Option<fn()>` isn't an
// `Option<fn() with panic>`.
fn nothing() {}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let f: Option<fn()> = Option::Some(nothing);
    let g: Option<fn() with panic> = f;
    out.line(&g.is_some())?;
    Result::Ok(())
}
// error: expected `Option<fn() -> () with panic>`, found `Option<fn() -> ()>`
