// Only an integer can be written in decimal with 0s in front.
fn main() {
    let s = "ab";
    let _t = f"{s:04}"; // error: `&'static str` isn't an integer type, which a hole `{x:05}` of a format string needs
}
