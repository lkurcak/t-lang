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

fn main() with alloc {
    match String::from_utf8(bytes(b"caf\xC3\xA9")) {
        Result::Ok(s) => {
            print_str(s.as_str()); // expect: café
            let l = s.into_bytes();
            print(l.len()); // expect: 5
            l.free();
        },
        Result::Err(l) => l.free(),
    }
    match String::from_utf8(bytes(b"caf\xC3")) {
        Result::Ok(s) => s.free(),
        Result::Err(l) => {
            print_str("not UTF-8"); // expect: not UTF-8
            print(l.len());         // expect: 4
            l.free();
        },
    }
}
