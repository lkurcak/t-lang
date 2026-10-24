// Overwriting a value drops the old one, here and through a `&mut`; a
// value moved out on one path only is dropped on the other.
use alloc::string::String;

fn replace_it(r: &mut String) with alloc {
    *r = String::from("through");
}

fn take(_s: String) {}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut s = String::from("first");
    s = String::from("second");
    replace_it(&mut s);
    out.line(&s.as_str())?; // expect: through
    let t = String::from("maybe");
    if s.len() > 3 {
        take(t);
    }
    Result::Ok(())
}
