// What the checker trusts, writing a byte, is private to std: a program
// writes text, through `Write`.
fn main(os: std::Os) {
    let mut out = os.stdout;
    let _code = out.write_byte(0xFF);
}
// error: private
