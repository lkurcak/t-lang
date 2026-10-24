// A doc comment before something that can't have one documents nothing.
// The fix makes it a plain comment.

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    /// Before a statement.
    // warning: unused doc comment
    let x: i64 = 1;
    out.line(&x)?; // expect: 1
    Result::Ok(())
    // warning: unused doc comment
    /// At the end of a block.
}

/// At the end of the file.
/// Two lines, one warning.
// warning: unused doc comment
