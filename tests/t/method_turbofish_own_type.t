// A method's type arguments can name the program's own types, as a
// function's can.
#[derive(Discard)]
struct Point { x: i64 }

#[derive(Discard)]
struct S {}

impl S {
    fn id<T>(&self, t: T) -> T { t }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let s = S {};
    let p = s.id::<Point>(Point { x: 3 });
    out.line(&p.x)?; // expect: 3
    Result::Ok(())
}
