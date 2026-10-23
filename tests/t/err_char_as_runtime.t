// What isn't proved is checked as the program runs.
fn f(n: u32) -> char with panic { n as char }

fn main() with panic {
    print(f(0x41) as u32); // expect: 65
    f(0xD800);
}
// error: runtime error: `55296` isn't a character
