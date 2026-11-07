// Closing is private: a file is closed by dropping it, once, after which
// it's gone (lemma F2 of docs/48-files.md).
fn main(os: std::Os) with alloc {
    match os.fs.open("x") {
        Ok(f) => f.os_close(), // error: private
        Err(_) => {},
    }
}
