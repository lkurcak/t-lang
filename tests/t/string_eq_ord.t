// Strings compare by their characters, as `str`s do.
use alloc::string::String;

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let a = String::from("abc");
    let b = String::from("abd");
    let mut c = String::from("ab");
    c.push('c');
    let w = show(&mut out, &a, &b, &c);
    a.discard();
    b.discard();
    c.discard();
    w
}

fn show(out: &mut std::io::Stdout, a: &String, b: &String, c: &String) -> Result<(), std::io::Error> {
    out.line(&(a == c))?; // expect: true
    out.line(&(a == b))?; // expect: false
    out.line(&(a != b))?; // expect: true
    out.line(&a.lt(b))?; // expect: true
    out.line(&b.lt(a))?; // expect: false
    out.line(&a.le(c)) // expect: true
}
