# Step 48: files

Code: `std/fs.t` (`Fs`, `ReadFile`), `std/io.t` (`Stdin`'s new
`read_to_end` and `read_to_string`, and the shared decoding of failures),
`core/io.t` (`ErrorKind::InvalidInput`), the runtime's `t_file_*` in
`src/cgen/hosted.h`, and the interpreter's files in `Terminal`
(`src/interp.rs`) and `Process` (`src/lib.rs`).

The second of the three steps that fill in the `Os`, after bytes,
arguments and the environment (docs/47-io.md). t-nonograms wants to read
the puzzle its command line names; a program that saves anything wants to
write a file. This step gives a program files, and as little else as
that takes: a file is written whole, by one call, and read whole or bit
by bit.

## The file system is a capability

The `Os` that `main` is given has one more field, `fs: Fs`. It's the
right to reach the file system, as `threads` is the right to start
threads: a function that's handed no `Fs` can't open a file, so what it
does depends on its arguments alone (lemma C1 of docs/19-std-io.md, as
before).

```rust
use std::io::Error;

fn main(os: std::Os) -> Result<(), Error> with alloc {
    let fs = os.fs;
    let mut out = os.stdout;
    fs.write("notes.txt", "one\n".as_bytes())?;
    fs.append("notes.txt", "two\n".as_bytes())?;
    let s = fs.read_to_string("notes.txt")?;
    out.put(&s.as_str())?; // expect: one
    s.discard();           // expect: two
    fs.remove("notes.txt")
}
```

`Fs` isn't `Copy`, and its methods take `&self`: a function that needs
files takes `&Fs`. It's `Sync`, so threads can share one. Copying it
would be sound too, as nothing about the order of opening files is
promised, but `Copy` can be added later without breaking anything, and
not taken away (decision G4).

A path is a `&str`: relative to the directory the program was started
in, or from `/`. One with a `\0` in it is an error of the new kind
`InvalidInput`, before it reaches the operating system, which would read
it only up to the `\0`: a file of another name.

## Whole files

What a program writes, it writes whole:

| Method | What it does |
|---|---|
| `fs.write(path, bytes)` | the file holds `bytes`, and nothing else: made if there's none |
| `fs.append(path, bytes)` | `bytes` added to its end: made if there's none |
| `fs.read(path)` | all of it, a `List<u8>` (`with alloc`) |
| `fs.read_to_string(path)` | all of it, a `String`, or `InvalidData` if it isn't UTF-8 |
| `fs.remove(path)` | it's gone: `NotFound` if there was none |

Each is one call that opens the file, does it, closes it, and gives back
one `Result`. What's to be written is built first, in a `String` or a
`List<u8>`, which are each a `Write` (docs/47-io.md):

```rust
use alloc::list::List;
use core::io::Write;
use std::io::Error;

fn main(os: std::Os) -> Result<(), Error> with alloc {
    let mut out = os.stdout;
    let mut report = List::<u8>::new();
    let mut i = 1u32;
    while i <= 3 {
        report.line(&i)?;
        i = core::num::wrapping_add(i, 1);
    }
    os.fs.write("report.txt", report.as_slice())?;
    report.discard();
    let back = os.fs.read("report.txt")?;
    out.line(&back.len())?; // expect: 6
    back.discard();
    Ok(())
}
```

## Reading bit by bit

`fs.open(path)` gives a `ReadFile`, which is what `Stdin` is, for a file:
a `core::io::Read`, read through a buffer, with the same `read_line`,
`read_to_end` and `read_to_string`. Code generic over a `Read` takes
either:

```rust
use alloc::string::String;
use core::io::Read;
use std::io::Error;

fn lines<R: Read>(r: &mut R) -> Result<u32, Error> with {alloc, R::e} {
    let mut line = String::new();
    let mut n = 0u32;
    while alloc::io::read_line(r, &mut line)? {
        n = core::num::wrapping_add(n, 1);
    }
    line.discard();
    Ok(n)
}

fn main(os: std::Os) -> Result<(), Error> with alloc {
    let mut out = os.stdout;
    let mut input = os.stdin;
    let mut f = os.fs.open("puzzle.txt")?;
    out.line(&lines(&mut f)?)?;   // expect: 3
    out.line(&lines(&mut input)?) // expect: 1
}
// file: puzzle.txt=1 2
// file: puzzle.txt=3
// file: puzzle.txt=4 5 6
// stdin: solve
```

Where they differ is only at the edges. A `ReadFile` is opened, which can
fail; `Stdin` is open already. There's one `Stdin`, whose buffer and
place in the input are the process's, so it's unique (lemma C2 of
docs/19-std-io.md); each `ReadFile` has its own, so opening a file twice
gives two that don't disturb each other. A `ReadFile` is never a
terminal, so has no `is_terminal`, and reading it doesn't write the
output out first, as reading `Stdin` does for a prompt.

Opening is `with alloc`, as the buffer is memory: if there's none, the
program stops, as when a `List` can't grow.

## Dropping a file closes it

A `ReadFile` is `Discard`, with a `discard` of its own (docs/20-destructors.md),
which closes it. Closing a file that's only been read can't lose
anything, so it doesn't matter whether `close` worked, and the implicit
call is total, as an inserted call must be (lemma D2 of
docs/20-destructors.md).

**Lemma F1 (no forged files).** `Fs` and `ReadFile` are intrinsic types,
with no fields: no code makes one (`std::fs::ReadFile {}` is an error).
The one `Fs` is in the `Os`, and a `ReadFile` comes only from an
`Fs::open` that worked, so each holds a file the operating system opened.

```rust,error
fn main() {
    let f = std::fs::ReadFile {}; // error: `ReadFile` is intrinsic: only its own functions make one or take one apart
    f.discard();
}
```

**Lemma F2 (each file is closed once, and not used after).** A
`ReadFile` isn't `Copy`, and its intrinsic `os_close` is private, called
only by its `discard`, which takes it by value. Moving it is the only way
to hand it on, so exactly one value holds each open file, and when it's
dropped, it's closed and gone: no read after, no second close, and no
read of a descriptor the operating system has since given another file.

```rust,error
use alloc::string::String;

fn main(os: std::Os) with alloc {
    if let Ok(f) = os.fs.open("x") {
        let mut f = f;
        f.discard();
        let mut s = String::new();
        let _ = f.read_line(&mut s); // error: moved
        s.discard();
    }
}
```

The interpreter checks F2 as it runs, as it checks that every box is
freed: a program that ends with a file still open fails with "file leak".

**Lemma F3 (no data races).** A `ReadFile`'s buffer is reached only
through `&mut ReadFile` or by its `discard`, so by one thread at a time;
it's `Send` and `Sync`. `Fs` keeps nothing, and opening is the operating
system's, safe from any thread.

## Writing is whole, for now

There's no file to write bit by bit: no `WriteFile`. That's on purpose.
A file being written holds what's not yet written out, so dropping one
must not be implicit: its `close` can fail, and a failure no one sees is
a lost write. So it would be linear, as `JoinHandle` is, and then `?`
couldn't return while one is open, as it can't while a thread runs. There
are ways round that (a method that closes and gives back the first
error; a `try` block; letting `?` close it when it returns an `Err`, as
Zig's `errdefer` does), and each is a promise for good. Whole-file writes
have none of these questions: there's never a file half written that the
program holds. They're what most programs want, and a library can build
on them. A `WriteFile` comes when a program needs to stream what it
writes: a log, or more than fits in memory.

## What the checker trusts

| Intrinsic | Promise |
|---|---|
| `Fs::os_open(path)` | `path` has no `\0`; gives a `ReadFile` holding a newly opened file, or a failure |
| `Fs::os_write(path, bytes, append)` | reads only `path` and `bytes`; gives 0 or a failure, the file closed either way |
| `Fs::os_remove(path)` | reads only `path`; 0 or a failure |
| `ReadFile::os_read(buf)` | writes at most `buf.len()` bytes into `buf`, as `Stdin::os_read` |
| `ReadFile::os_close()` | closes the file and frees its buffer |

In C, a `ReadFile` is a pointer to a `struct t_file`, the descriptor and
its buffer, on the heap. The interpreter keeps the files a test starts
with in memory, by path, in one directory (a test says them with
`// file: path=line`); `t run` uses the real ones. The differential tests
run each compiled program in a new directory holding them.

## Design decisions worth noticing

- **Whole files first.** Most files a program touches fit in memory:
  settings, saves, documents, puzzles. One call has fewer ways to go
  wrong than a handle, as there's no state between calls for a program to
  get wrong. Streaming is still the right tool for what's bigger than
  memory, what never ends (a pipe), or what must reach the disk as it
  happens (a log): reading streams now, writing later.
- **`ReadFile`, not `File`** (G3): what a handle does decides whether it
  can be dropped, so its name says it. A `WriteFile` would be its linear
  sibling.
- **The buffer is the runtime's** (G2), as `Stdin`'s is: opening needs no
  `List`, and `read_line` on a file is quick. A `BufReader` in T, generic
  over any `Read`, can come with a `BufRead` trait.
- **Paths are `&str`**, as arguments are (decision D4 of docs/47-io.md).
- **The whole file system, for now** (D2 of the Os design pass): later,
  an `Fs` can hand out a capability for one directory and what's under it,
  as WASI does.

## Deliberately missing (coming later)

| Missing | Why it's missing | Step |
|---|---|---|
| `WriteFile`, writing bit by bit | linear, and how `?` meets it isn't decided | when needed |
| replacing a file atomically (`fs.replace`) | write a new file, then rename it over the old | — |
| directories, metadata, seeking | not needed yet | — |
| capabilities for one directory | `Fs` is the whole file system (D2) | — |
| paths that aren't UTF-8 | a `Path` type | — |
| clocks, randomness, raw terminal mode | each its own capability in the `Os` | Os-C |

## Exercises

1. Write `fn copy(fs: &Fs, from: &str, to: &str) -> Result<usize, Error>`
   with whole-file calls. What does it need that a streaming copy
   wouldn't?
2. Why can a `ReadFile` be dropped, but a file being written couldn't be?
   What would a `WriteFile` whose `discard` flushed and ignored failures
   break?
3. Write `fn tail(fs: &Fs, path: &str, n: usize)` that prints a file's
   last `n` lines, reading it once through a `ReadFile`, keeping only `n`
   lines in memory.
4. Two `ReadFile`s of the same file are read in turn. Why don't they
   disturb each other, while two `Stdin`s would?
5. `fs.write` truncates the file, then writes. What does a reader see if
   the program dies halfway? How would `fs.replace` avoid it?

## Reading

- WASI, *the WebAssembly System Interface*: files as capabilities, and
  directories handed to a program rather than a whole file system.
- Zig's `defer` and `errdefer`, and Linear Haskell's `RIO` (in
  `linear-base`): two answers to closing what's linear when a function
  fails.
- Go's `os.ReadFile`/`os.WriteFile` and Rust's `std::fs::read_to_string`/
  `write`: the whole-file calls of other standard libraries.
- *Files are hard* (Dan Luu, 2015), on what it takes for a write to
  survive a crash: why `replace` is its own step.
