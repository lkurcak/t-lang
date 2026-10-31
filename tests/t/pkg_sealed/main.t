// Another package's type can't be `q::Small`, so it can have a
// `q::Describe` of its own beside `q`'s blanket impl.
use q::Describe;

#[derive(Discard)]
struct Mine {}

impl Describe for Mine {
    fn describe(&self) -> i64 { 2 }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&(Mine {}).describe())?; // expect: 2
    out.line(&7u8.describe())?;       // expect: 1
    Result::Ok(())
}
