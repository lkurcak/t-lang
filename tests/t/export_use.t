// An `export use` in an exported module is a way for other packages to
// name what's exported in a module that isn't.
mod m {
    export fn f() -> i64 { 1 }

    #[derive(Discard)]
    export struct P { export x: i64 }

    impl P {
        export fn get(&self) -> i64 { self.x }
    }
}

export use m::{f, P};

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&(f() + P { x: 2 }.get()))?; // expect: 3
    Result::Ok(())
}
