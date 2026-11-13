// Only an integer can be written in hexadecimal.
fn main() {
    let b = true;
    let _s = f"{b:x}"; // error: `bool` isn't an integer type, which a hole `{x:x}` of a format string needs
}
