trait Show {
    fn show(&self) -> i64;
}

fn f() -> impl Show { 1i64 }

fn main(os: std::Os) -> Result<(), std::io::Error> { let mut out = os.stdout; out.line(&f().show())?;  Result::Ok(()) }
// error: `i64` doesn't implement `Show`, which `f`'s return type says
