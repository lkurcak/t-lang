// Variables bound by patterns can be `mut` too.

#[derive(Discard)]
struct P { x: i64, y: i64 }

#[allow(dead_code)]
#[derive(Discard)]
enum Option<T> { Some(T), None }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let P { x: mut a, y } = P { x: 1, y: 2 };
    a += y;
    out.line(&a)?;                       // expect: 3
    match Option::Some(5i64) {
        Option::Some(mut n) => {
            n *= 2;
            out.line(&n)?;               // expect: 10
        },
        Option::None => {},
    }
    Result::Ok(())
}
