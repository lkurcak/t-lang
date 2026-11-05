mod a {
    pub use super::b::x;
}

mod b {
    pub use super::a::x;
}

fn main(os: std::Os) -> Result<(), std::io::Error> { let mut out = os.stdout; out.line(&a::x())?;  Ok(()) }
// error: these imports go round in a circle
