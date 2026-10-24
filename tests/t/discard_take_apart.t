// The module a type with a `discard` of its own is declared in may take
// its values apart, and then nothing calls it: `close` hands back what
// `discard` would have thrown away.
mod file {
    pub struct File<'a> { pub out: &'a mut std::io::Stdout, pub fd: i64 }

    impl<'a> Discard for File<'a> {
        fn discard(self) {
            let _ = self.out.line(&"discarded");
        }
    }

    impl<'a> File<'a> {
        pub fn close(self) -> i64 {
            let File { out: _, fd } = self;
            fd
        }
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let fd = file::File { out: &mut out, fd: 7 }.close();
    {
        let _f = file::File { out: &mut out, fd: 8 };
    } // expect: discarded
    out.line(&fd) // expect: 7
}
