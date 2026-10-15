// A doc comment before something that can't have one documents nothing.
// The fix makes it a plain comment.

fn main() {
    /// Before a statement.
    // warning: unused doc comment
    let x = 1;
    print(x); // expect: 1
    // warning: unused doc comment
    /// At the end of a block.
}

/// At the end of the file.
/// Two lines, one warning.
// warning: unused doc comment
