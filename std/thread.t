//! Threads: functions that run at the same time as the one that starts
//! them, each its own way through the program (see docs/41-threads.md).
//!
//! A thread is started with `Threads::spawn`, which needs the program's
//! `Threads`, from its `Os`: starting threads is something a function can
//! do only if it's handed the right to. It gives back a handle, which must
//! be joined, as it can't be dropped: `join` waits for the thread to end,
//! and gives back what it did. So a thread may borrow what the function
//! that starts it can reach, for as long as the handle lives: it's over
//! before what it borrows is gone.

/// The right to start threads: what `fn main(os: Os)` is given in its
/// `os.threads`. It's `Sync`, so a `&Threads` may go to a thread, which
/// can then start threads of its own.
#[derive(Discard, Send, Sync)]
#[intrinsic]
export struct Threads {}

/// A thread that's running, or has ended, and what its closure gives back,
/// a `R`, once it has. It lives no longer than `'a`, which is no longer
/// than anything the thread borrows.
///
/// It isn't `Discard`: it must be joined. That's what makes borrowing
/// sound, as no handle is ever dropped, nor forgotten, while its thread
/// runs (lemma Z1 of docs/41-threads.md).
#[derive(Send where R: Send)]
#[intrinsic]
export struct JoinHandle<'a, R> {}

impl Threads {
    /// Starts a thread that runs `f`, and gives back its handle.
    ///
    /// What `f` may do, `spawn` does: a function that says it can't panic
    /// can't start a thread that may. And starting it allocates the
    /// thread's stack, so it's `with alloc`: if the operating system can't
    /// start one, the program stops, as when the memory runs out.
    ///
    /// `f` and what it gives back go from one thread to another, so they
    /// must be `Send`; and they must live as long as the handle does.
    export fn spawn<'a, R: Send + 'a, effect e, F: FnOnce() -> R with e + Send + 'a>(
        &self,
        f: F,
    ) -> JoinHandle<'a, R> with {alloc, e} {
        start(f, run)
    }
}

impl<'a, R> JoinHandle<'a, R> {
    /// Waits for the thread to end, and gives back what its closure did.
    #[intrinsic]
    export fn join(self) -> R;
}

/// What a new thread runs: `f`.
fn run<R, effect e, F: FnOnce() -> R with e>(f: F) -> R with e { f() }

/// Starts a thread that runs `run(f)`.
#[intrinsic]
fn start<'a, R: 'a, effect e, F: 'a>(f: F, run: fn(F) -> R with e) -> JoinHandle<'a, R> with {
    alloc,
    e,
};
