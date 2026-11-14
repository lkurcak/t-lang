// A `move` closure that names a field of a value it can't take apart, one
// with a `discard` of its own, takes the whole value: moving the field
// out would skip the `discard`.
mod file {
    pub struct File { pub name: alloc::string::String, pub fd: i64 }

    impl Discard for File {
        fn discard(self) {}
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let f = file::File { name: alloc::string::String::from("log"), fd: 3 };
    let len = move || f.name.len();
    out.line(&len()) // expect: 3
}
