// Reading input a line at a time: the `String` is reused, and freed once,
// by `main`, which owns it, while `echo` does the reading and writing,
// with `?` passing errors on.
use alloc::string::String;
use std::io::{Error, Stdin, Stdout};
use std::Os;

fn echo(input: &mut Stdin, out: &mut Stdout, line: &mut String) -> Result<i64, Error> with alloc {
    let mut n = 0;
    while input.read_line(line)? {
        n = checked_add(n, 1).unwrap_or(n);
        out.put(&"[")?;
        out.put(&line.as_str())?;
        out.line(&"]")?;
    }
    Result::Ok(n)
}

fn main(os: Os) -> Result<(), Error> with alloc {
    let Os { stdin: mut input, stdout: mut out, .. } = os;
    let mut line = String::new();
    let echoed = echo(&mut input, &mut out, &mut line);
    line.discard();
    out.line(&echoed?)
}
// stdin: hello
// stdin: wörld
// stdin:
// stdin: last
// expect: [hello]
// expect: [wörld]
// expect: []
// expect: [last]
// expect: 4
