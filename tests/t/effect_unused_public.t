// An effect said but not had is a warning only where nothing outside the
// package can count on it: an exported function may say one to keep the
// right to have it later, and a trait's method says what its impls may do.
pub trait Store {
    fn get(&self) -> i64 with panic;

    fn twice(&self) -> i64 with panic { self.get() }
}

#[derive(Discard)]
struct Fixed {}

impl Store for Fixed {
    fn get(&self) -> i64 with panic { 1 }
}

export fn reserved() -> i64 with panic { 1 }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&(Fixed {}.twice() + reserved()))?; // expect: 2
    Result::Ok(())
}
