// `Box` isn't a reserved name: without `use alloc::boxed::Box`, a program's
// own `Box` is an ordinary struct.
#[derive(Discard)]
struct Box { x: i64 }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let b = Box { x: 3 };
    out.line(&b.x)?; // expect: 3
    Result::Ok(())
}
