// A closure captures what its body uses, as it is when the closure is made,
// and only the variables in scope there, not ones of the same name that its
// own parameters, `let`s or patterns shadow.
#[derive(Discard)]
enum Opt { None, Some(i64) }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let a = 1;
    let b = 2;
    let f = |b| {
        let a = a * 10;
        match Opt::Some(b) {
            Opt::Some(b) => a + b,
            Opt::None => 0,
        }
    };
    let a = 100;
    let _unused = Opt::None;
    out.line(&(f(5) + a))?;                // expect: 115
    // Nested closures capture through the outer one.
    let g = || { let h = || b; h() };
    out.line(&g())?;                     // expect: 2
    // Top-level functions aren't captured; they're always there.
    let h = || double(b);
    out.line(&h())?;                     // expect: 4
    Result::Ok(())
}

fn double(x: i64) -> i64 with panic { x * 2 }
