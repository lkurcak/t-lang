// `from_utf8` takes a list of bytes if they're UTF-8, and gives it back if
// they're not; `into_bytes` gives the list.
use alloc::list::List;
use alloc::string::String;

fn bytes(b: &[u8]) -> List<u8> with alloc {
    let mut l = List::new();
    for x in b.iter() {
        l.push(*x);
    }
    l
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    match String::from_utf8(bytes(b"caf\xC3\xA9")) {
        Result::Ok(s) => {
            let w = out.line(&s.as_str()); // expect: café
            let l = s.into_bytes();
            let n = l.len();
            l.free();
            w?;
            out.line(&n)?; // expect: 5
        },
        Result::Err(l) => l.free(),
    }
    match String::from_utf8(bytes(b"caf\xC3")) {
        Result::Ok(s) => s.free(),
        Result::Err(l) => {
            let n = l.len();
            l.free();
            out.line(&"not UTF-8")?; // expect: not UTF-8
            out.line(&n)?; // expect: 4
        },
    }
    Result::Ok(())
}
