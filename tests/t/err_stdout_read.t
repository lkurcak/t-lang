// The output isn't read from.
use core::io::Read;

fn main(os: std::Os) {
    let mut out = os.stdout;
    let mut buf = [0u8; 4];
    let _ = out.read(buf.as_mut_slice()); // error: doesn't implement it
}
