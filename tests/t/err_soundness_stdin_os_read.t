// A stream's intrinsics are private: `read` checks what `os_read` gives.
fn main(os: std::Os) {
    let mut input = os.stdin;
    let mut buf = [0u8; 4];
    let _ = input.os_read(buf.as_mut_slice()); // error: private
}
