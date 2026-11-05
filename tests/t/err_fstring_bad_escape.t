// A format string's text has a string literal's escapes, and no others.
fn main() {
    let _ = f"a\qb";
}
// error: unknown escape `\q`
