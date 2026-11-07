//! The terminal, for real: programs run on a pseudo-terminal, compiled and
//! interpreted, that put it in raw mode, read keys, and stop in each way a
//! program can, after which the terminal must be as it was (lemma T4 of
//! docs/49-time-entropy-terminal.md).
#![cfg(unix)]

use std::os::fd::{FromRawFd, OwnedFd, RawFd};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

/// Reads a key in raw mode, after showing the terminal's size, and whether
/// a key came before a deadline 50 ms off; then stops as the key says: `p`
/// panics, `e` gives back an error, anything else returns. It says
/// `ready` once raw, and what it read.
const PROGRAM: &str = r#"
use core::io::Read;
use core::time::Duration;
use std::io::{Error, ErrorKind};

fn main(os: std::Os) -> Result<(), Error> {
    let mut out = os.stdout;
    let mut input = os.stdin;
    let clock = os.clock;
    let s = out.size()?;
    out.line(&f"size {s.cols}x{s.rows}")?;
    let mut raw = input.raw()?;
    out.line(&"ready")?;
    let early = raw.wait(clock.now().add(Duration::from_millis(50)))?;
    out.line(&f"early {early}")?;
    let mut b = [0u8; 1];
    let n = raw.read(b.as_mut_slice())?;
    out.line(&f"got {n} {b[0]}")?;
    let ready = raw.wait(clock.now())?;
    out.line(&f"more {ready}")?;
    if b[0] == 112u8 {
        core::panic::panic("boom");
    }
    if b[0] == 101u8 {
        raw.discard();
        return Err(Error::new(ErrorKind::Other));
    }
    raw.discard();
    out.line(&"done")
}
"#;

/// A pseudo-terminal of 80 columns and 24 rows: the side a program is
/// given, and the side that types into it and reads what it writes.
struct Pty {
    master: OwnedFd,
    slave: OwnedFd,
}

impl Pty {
    fn new() -> Pty {
        let (mut master, mut slave) = (0, 0);
        let size = libc::winsize {
            ws_row: 24,
            ws_col: 80,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        // SAFETY: two out-pointers, and a `winsize` it only reads.
        let r = unsafe {
            libc::openpty(
                &mut master,
                &mut slave,
                std::ptr::null_mut(),
                std::ptr::null(),
                &size,
            )
        };
        assert_eq!(r, 0, "openpty: {}", std::io::Error::last_os_error());
        // SAFETY: both were just opened, and are owned here alone.
        unsafe {
            Pty {
                master: OwnedFd::from_raw_fd(master),
                slave: OwnedFd::from_raw_fd(slave),
            }
        }
    }

    fn fd(f: &OwnedFd) -> RawFd {
        std::os::fd::AsRawFd::as_raw_fd(f)
    }

    /// The terminal's local modes: whether it's in line mode, and echoes.
    fn modes(&self) -> (bool, bool) {
        // SAFETY: one out-pointer.
        unsafe {
            let mut t: libc::termios = std::mem::zeroed();
            assert_eq!(libc::tcgetattr(Self::fd(&self.slave), &mut t), 0);
            (t.c_lflag & libc::ICANON != 0, t.c_lflag & libc::ECHO != 0)
        }
    }

    /// Starts `command` on the terminal: it's the input, the output, and
    /// where errors go.
    fn spawn(&self, mut command: Command) -> Child {
        let io = || Stdio::from(self.slave.try_clone().unwrap());
        command
            .stdin(io())
            .stdout(io())
            .stderr(io())
            .spawn()
            .unwrap()
    }

    /// Reads what's been written to the terminal onto `seen`, until it
    /// holds `needle` or `wait` has passed.
    fn read_until(&self, seen: &mut String, needle: &str, wait: Duration) {
        let end = Instant::now() + wait;
        while !seen.contains(needle) && Instant::now() < end {
            let mut p = libc::pollfd {
                fd: Self::fd(&self.master),
                events: libc::POLLIN,
                revents: 0,
            };
            // SAFETY: one `pollfd`; a buffer as long as it says.
            unsafe {
                if libc::poll(&mut p, 1, 20) > 0 {
                    let mut buf = [0u8; 1024];
                    let n = libc::read(Self::fd(&self.master), buf.as_mut_ptr().cast(), buf.len());
                    if n > 0 {
                        seen.push_str(&String::from_utf8_lossy(&buf[..n as usize]));
                    }
                }
            }
        }
    }

    /// Types `bytes`.
    fn press(&self, bytes: &[u8]) {
        // SAFETY: a buffer as long as it says.
        let n = unsafe { libc::write(Self::fd(&self.master), bytes.as_ptr().cast(), bytes.len()) };
        assert_eq!(n as usize, bytes.len());
    }
}

/// How a run went: what it wrote, and how it ended.
struct Ran {
    seen: String,
    status: std::process::ExitStatus,
}

/// Runs the program, compiled or not, on a new terminal: once it's raw,
/// `act` does something to it; then it must end, and the terminal must be
/// back in line mode, echoing.
fn run(compiled: bool, act: impl FnOnce(&Pty, &Child)) -> Ran {
    let dir = std::env::temp_dir().join(format!(
        "t-terminal-{compiled}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("main.t");
    std::fs::write(&path, PROGRAM).unwrap();
    let command = if compiled {
        let exe = dir.join("main");
        let built: Output = Command::new(env!("CARGO_BIN_EXE_t"))
            .args(["build", "-o", exe.to_str().unwrap(), path.to_str().unwrap()])
            .output()
            .unwrap();
        assert!(built.status.success(), "{built:?}");
        Command::new(exe)
    } else {
        let mut c = Command::new(env!("CARGO_BIN_EXE_t"));
        c.arg("--interp").arg(&path);
        c
    };
    let pty = Pty::new();
    assert_eq!(pty.modes(), (true, true), "a new terminal is in line mode");
    let mut child = pty.spawn(command);
    let mut seen = String::new();
    pty.read_until(&mut seen, "ready", Duration::from_secs(60));
    assert!(seen.contains("ready"), "never got raw: {seen:?}");
    assert_eq!(
        pty.modes(),
        (false, false),
        "raw mode: no line mode, no echo"
    );
    act(&pty, &child);
    let end = Instant::now() + Duration::from_secs(30);
    let status = loop {
        if let Some(s) = child.try_wait().unwrap() {
            break s;
        }
        assert!(Instant::now() < end, "it never ended: {seen:?}");
        pty.read_until(&mut seen, "\u{0}", Duration::from_millis(20));
    };
    pty.read_until(&mut seen, "\u{0}", Duration::from_millis(50));
    assert_eq!(
        pty.modes(),
        (true, true),
        "the terminal is put back: {seen:?}"
    );
    let _ = std::fs::remove_dir_all(&dir);
    Ran { seen, status }
}

/// Waits a little, past the program's first 50 ms `wait`, so that it sees
/// none, then types `key`.
fn press_later(key: &'static [u8]) -> impl FnOnce(&Pty, &Child) {
    move |pty, _| {
        std::thread::sleep(Duration::from_millis(300));
        pty.press(key);
    }
}

fn both(check: impl Fn(bool)) {
    check(true);
    check(false);
}

#[test]
fn a_key_at_a_time_then_dropped() {
    both(|compiled| {
        let ran = run(compiled, press_later(b"q"));
        assert_eq!(ran.status.code(), Some(0), "{:?}", ran.seen);
        for line in [
            "size 80x24",
            "early false",
            "got 1 113",
            "more false",
            "done",
        ] {
            assert!(ran.seen.contains(line), "{line:?} not in {:?}", ran.seen);
        }
    });
}

#[test]
fn ctrl_c_is_a_key() {
    both(|compiled| {
        let ran = run(compiled, press_later(b"\x03"));
        assert_eq!(ran.status.code(), Some(0), "{:?}", ran.seen);
        assert!(ran.seen.contains("got 1 3"), "{:?}", ran.seen);
    });
}

#[test]
fn keys_waiting_are_seen() {
    both(|compiled| {
        let ran = run(compiled, press_later(b"qq"));
        assert!(ran.seen.contains("more true"), "{:?}", ran.seen);
    });
}

#[test]
fn put_back_after_a_panic() {
    both(|compiled| {
        let ran = run(compiled, press_later(b"p"));
        assert_eq!(ran.status.code(), Some(101), "{:?}", ran.seen);
        assert!(ran.seen.contains("boom"), "{:?}", ran.seen);
    });
}

#[test]
fn put_back_after_an_error() {
    both(|compiled| {
        let ran = run(compiled, press_later(b"e"));
        assert_eq!(ran.status.code(), Some(1), "{:?}", ran.seen);
    });
}

#[test]
fn put_back_when_terminated() {
    both(|compiled| {
        let ran = run(compiled, |_, child| {
            // SAFETY: a signal to a child that hasn't been waited for.
            unsafe { libc::kill(child.id() as libc::pid_t, libc::SIGTERM) };
        });
        use std::os::unix::process::ExitStatusExt;
        assert_eq!(ran.status.signal(), Some(libc::SIGTERM), "{:?}", ran.seen);
    });
}
