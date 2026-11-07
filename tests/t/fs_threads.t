// `Fs` is `Sync`: threads given `&Fs` each read a file of their own, and a
// `ReadFile` is `Send`, so one opened here can be read there.
use std::fs::{Fs, ReadFile};
use std::io::Error;

fn size(fs: &Fs, path: &str) -> usize with alloc {
    match fs.read(path) {
        Ok(b) => {
            let n = b.len();
            b.discard();
            n
        },
        Err(_) => 0,
    }
}

fn lines(f: ReadFile) -> u32 with alloc {
    let mut f = f;
    let mut line = alloc::string::String::new();
    let mut n = 0u32;
    while let Ok(true) = f.read_line(&mut line) {
        n = core::num::wrapping_add(n, 1);
    }
    line.discard();
    n
}

fn main(os: std::Os) -> Result<(), Error> with alloc {
    let fs = &os.fs;
    let mut out = os.stdout;
    // Opened first: a `?` can't return while a thread runs, as its
    // handle must be joined.
    let f = fs.open("b.txt")?;
    let a = os.threads.spawn(|| size(fs, "a.txt"));
    let b = os.threads.spawn(|| size(fs, "b.txt"));
    let c = os.threads.spawn(move || lines(f));
    let na = a.join();
    let nb = b.join();
    let nc = c.join();
    out.line(&na)?; // expect: 4
    out.line(&nb)?; // expect: 6
    out.line(&nc)   // expect: 3
}
// file: a.txt=abc
// file: b.txt=x
// file: b.txt=y
// file: b.txt=z
