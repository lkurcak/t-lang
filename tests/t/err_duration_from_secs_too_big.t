// The limit is checked for a literal, too.
fn main() {
    let _ = core::time::Duration::from_secs(18_446_744_074); // error: MAX_SECS
}
