//! Threads, as the interpreter runs them (docs/41-threads.md, I): each on
//! a thread of the machine's, with a stack of its own, but only one at a
//! time, which holds the *baton*. It's handed on at the points where what
//! one thread does can be seen by another: each atomic operation, each
//! `spin` (a mutex's, a barrier's, a program's own), and a `join` of a
//! thread that hasn't ended, as well as when a thread ends. Each time to
//! the next thread that can run, in the order they were started, round
//! and round: so a run is the same every time, and a thread that waits
//! for another, spinning, lets it run.
//!
//! Between those points, a thread runs alone: so what isn't atomic is
//! never seen half done, as a program the checker accepted has no data
//! races (lemma Z2), which a schedule like this one can't tell apart.
//!
//! What's each thread's own of the interpreter (`Saved`) is swapped in and
//! out of it at each handover; the rest, the memory above all, is shared.

use super::{Blame, Interp, Site, Value, unsound};
use crate::error::{Error, Span};
use std::sync::{Arc, Condvar, Mutex};

/// What's a thread's own of the interpreter, kept while another runs.
pub(super) struct Saved<'p> {
    calls: Vec<(Span, Blame)>,
    frames: Vec<String>,
    sites: Vec<Site<'p>>,
    depth: usize,
    message: String,
}

/// Where a thread is.
enum Status {
    /// Running, or ready to.
    Ready,
    /// Waiting for thread `.0` to end.
    Joining(usize),
    /// Ended, with what its closure gave back, which `join` hasn't taken
    /// yet.
    Done(Value),
    /// Ended and joined, or aborted.
    Over,
}

struct Thread<'p> {
    status: Status,
    /// What's its own, while another thread runs.
    saved: Option<Saved<'p>>,
    /// The machine's thread it runs on, but for the first.
    os: Option<std::thread::JoinHandle<()>>,
}

/// The program's threads: the first is `main`'s.
pub(super) struct Threads<'p> {
    list: Vec<Thread<'p>>,
    /// The one that holds the baton.
    current: usize,
    /// Which thread may run: each waits on it until it's its own number.
    baton: Arc<(Mutex<usize>, Condvar)>,
    /// What stopped the program, once one has: then each thread, as it
    /// gets the baton, gives up, and hands it back to the first, which
    /// waits for them all to be gone.
    failed: Option<Error>,
    /// The interpreter's own panic, a bug's (`unsound`), in a thread: then
    /// the program has stopped (`failed`), and once every thread is gone,
    /// `wind_down` panics with it, on the first.
    panicked: Option<Box<dyn std::any::Any + Send>>,
}

impl Default for Threads<'_> {
    fn default() -> Self {
        Threads {
            list: vec![Thread {
                status: Status::Ready,
                saved: None,
                os: None,
            }],
            current: 0,
            baton: Arc::new((Mutex::new(0), Condvar::new())),
            failed: None,
            panicked: None,
        }
    }
}

/// What's handed to the machine's thread a T thread runs on: the
/// interpreter, and the values the thread starts with. The interpreter is
/// shared by all of them, but only the one holding the baton ever touches
/// it, or a value, and each handover goes through the baton's mutex, so
/// what one wrote, the next sees.
struct Shared<T>(T);

// SAFETY: see `Shared`: one thread at a time reaches what's in it.
unsafe impl<T> Send for Shared<T> {}

impl<'p, 'o> Interp<'p, 'o> {
    /// What's the running thread's own, taken out of the interpreter.
    fn save(&mut self) -> Saved<'p> {
        Saved {
            calls: std::mem::take(&mut self.calls),
            frames: std::mem::take(&mut self.frames),
            sites: std::mem::take(&mut self.sites),
            depth: std::mem::take(&mut self.depth),
            message: std::mem::take(&mut self.message),
        }
    }

    /// Puts thread `t`'s own back in the interpreter.
    fn restore(&mut self, t: usize) {
        let Some(s) = self.threads.list[t].saved.take() else {
            unsound("a thread to run with nothing of its own saved")
        };
        self.calls = s.calls;
        self.frames = s.frames;
        self.sites = s.sites;
        self.depth = s.depth;
        self.message = s.message;
    }

    /// Can thread `t` run?
    fn ready(&self, t: usize) -> bool {
        match self.threads.list[t].status {
            Status::Ready => true,
            Status::Joining(of) => matches!(self.threads.list[of].status, Status::Done(_)),
            Status::Done(_) | Status::Over => false,
        }
    }

    /// The next thread after the running one that can run, round and
    /// round: maybe the running one itself.
    fn next_ready(&self) -> Option<usize> {
        let n = self.threads.list.len();
        (1..=n)
            .map(|k| (self.threads.current + k) % n)
            .find(|&t| self.ready(t))
    }

    /// Hands the baton to thread `to`, with its own put back in the
    /// interpreter, and waits for it to come back.
    fn switch(&mut self, to: usize) {
        let me = self.threads.current;
        self.threads.list[me].saved = Some(self.save());
        self.hand_to(to);
        let baton = Arc::clone(&self.threads.baton);
        let (lock, cv) = &*baton;
        let mut holder = lock.lock().unwrap_or_else(|e| e.into_inner());
        while *holder != me {
            holder = cv.wait(holder).unwrap_or_else(|e| e.into_inner());
        }
    }

    /// Makes `to` the running thread, and wakes it.
    fn hand_to(&mut self, to: usize) {
        self.restore(to);
        self.threads.current = to;
        let (lock, cv) = &*self.threads.baton;
        *lock.lock().unwrap_or_else(|e| e.into_inner()) = to;
        cv.notify_all();
    }

    /// The error each thread gives up with, once the program has stopped.
    fn given_up(&self) -> Result<(), Error> {
        match &self.threads.failed {
            Some(e) => Err(e.clone()),
            None => Ok(()),
        }
    }

    /// Lets the next thread that can run have a turn, at a point where
    /// what it does can be seen: an atomic operation, or a spin.
    pub(super) fn turn(&mut self) -> Result<(), Error> {
        if self.threads.list.len() > 1
            && let Some(to) = self.next_ready()
            && to != self.threads.current
        {
            self.switch(to);
        }
        self.given_up()
    }

    /// `std::thread::start(f, run)`: a thread that runs `run(f)`, started
    /// with what the running one has of its own, so its calls are blamed
    /// and its effects checked as if `start` made them (lemma Z4). It
    /// waits for the baton.
    pub(super) fn start(&mut self, args: Vec<Value>, span: Span) -> Result<Value, Error> {
        self.effect("alloc");
        let [f, run] = <[Value; 2]>::try_from(args).unwrap_or_else(|_| unsound("arity"));
        let id = self.threads.list.len();
        let saved = Saved {
            calls: self.calls.clone(),
            frames: self.frames.clone(),
            sites: self.sites.clone(),
            depth: 0,
            message: String::new(),
        };
        let shared = Shared((self as *mut Self, f, run));
        let baton = Arc::clone(&self.threads.baton);
        let body = move || {
            // The whole of it, not its fields one by one, which aren't
            // `Send`.
            let shared = shared;
            let Shared((interp, f, run)) = shared;
            {
                let (lock, cv) = &*baton;
                let mut holder = lock.lock().unwrap_or_else(|e| e.into_inner());
                while *holder != id {
                    holder = cv.wait(holder).unwrap_or_else(|e| e.into_inner());
                }
            }
            // SAFETY: this thread holds the baton (`Shared`), and the
            // interpreter outlives it: `wind_down` and `join` wait for
            // every thread before it's dropped.
            let me: &mut Interp<'p, 'o> = unsafe { &mut *interp };
            let body = std::panic::AssertUnwindSafe(|| me.thread_body(id, f, run, span));
            if let Err(p) = std::panic::catch_unwind(body) {
                // It still holds the baton: hands it to the first, which
                // stops the others, and then panics with it.
                me.threads.list[id].status = Status::Over;
                me.stopped(p);
                me.hand_to(0);
            }
        };
        // SAFETY: the thread may borrow the interpreter, which isn't
        // `'static`, as it's joined before the interpreter goes (see
        // above).
        let os = unsafe {
            std::thread::Builder::new()
                .stack_size(crate::INTERP_STACK_SIZE)
                .spawn_unchecked(body)
        }
        .unwrap_or_else(|e| unsound(&format!("the interpreter couldn't start a thread: {e}")));
        self.threads.list.push(Thread {
            status: Status::Ready,
            saved: Some(saved),
            os: Some(os),
        });
        Ok(Value::Thread(id))
    }

    /// What thread `id` does, once it has the baton: runs `run(f)`, unless
    /// the program has stopped, and hands the baton on when it's over.
    fn thread_body(&mut self, id: usize, f: Value, run: Value, span: Span) {
        let ended = match self.given_up() {
            Ok(()) => self.call(run, vec![f], span),
            Err(e) => Err(e),
        };
        match ended {
            Ok(v) => self.threads.list[id].status = Status::Done(v),
            Err(e) => {
                self.threads.list[id].status = Status::Over;
                self.threads.failed.get_or_insert(e);
            }
        }
        let to = match self.threads.failed {
            Some(_) => 0,
            None => match self.next_ready() {
                Some(t) => t,
                None => {
                    let e = self.deadlock(span);
                    self.threads.failed = Some(e);
                    0
                }
            },
        };
        self.hand_to(to);
    }

    /// After the interpreter's panic `p`, in the running thread: the
    /// program stops, and once every thread has given up, `wind_down`
    /// panics with it.
    pub(super) fn stopped(&mut self, p: Box<dyn std::any::Any + Send>) -> Error {
        self.threads.panicked.get_or_insert(p);
        let e = Error::runtime(Span::new(0, 0), "the interpreter stopped");
        self.threads.failed.get_or_insert(e).clone()
    }

    /// The error when no thread can run: each waits for another to end.
    fn deadlock(&mut self, span: Span) -> Error {
        self.panic(
            span,
            "deadlock: every thread is waiting for another to end, which never will",
        )
    }

    /// `JoinHandle::join`: waits for thread `id` to end, letting the others
    /// run meanwhile, and gives back what it did.
    pub(super) fn join(&mut self, args: Vec<Value>, span: Span) -> Result<Value, Error> {
        let [Value::Thread(id)] = <[Value; 1]>::try_from(args).unwrap_or_else(|_| unsound("arity"))
        else {
            unsound("`join` of what isn't a thread")
        };
        let me = self.threads.current;
        loop {
            if let Status::Done(_) = self.threads.list[id].status {
                let Status::Done(v) =
                    std::mem::replace(&mut self.threads.list[id].status, Status::Over)
                else {
                    unreachable!()
                };
                if let Some(os) = self.threads.list[id].os.take() {
                    os.join().unwrap_or_else(|p| std::panic::resume_unwind(p));
                }
                return Ok(v);
            }
            if let Status::Over = self.threads.list[id].status {
                unsound("a thread joined twice");
            }
            self.threads.list[me].status = Status::Joining(id);
            let to = self.next_ready();
            match to {
                Some(to) if to != me => self.switch(to),
                Some(_) => {}
                None => return Err(self.deadlock(span)),
            }
            self.threads.list[me].status = Status::Ready;
            self.given_up()?;
        }
    }

    /// After `main`'s thread has ended, with `result`: if it ended by an
    /// error, or another thread's, every other thread must give up before
    /// the interpreter goes, as it runs on it. What stopped the program
    /// first is what's given back.
    /// The interpreter's own panic, in any thread, is panicked with again
    /// once they're all gone.
    pub(super) fn wind_down(&mut self, result: Result<Value, Error>) -> Result<Value, Error> {
        let result = self.wound_down(result);
        match self.threads.panicked.take() {
            Some(p) => std::panic::resume_unwind(p),
            None => result,
        }
    }

    fn wound_down(&mut self, result: Result<Value, Error>) -> Result<Value, Error> {
        if self.threads.list.len() == 1 {
            return result;
        }
        if let Err(e) = &result {
            self.threads.failed.get_or_insert_with(|| e.clone());
        }
        for t in 1..self.threads.list.len() {
            if matches!(
                self.threads.list[t].status,
                Status::Ready | Status::Joining(_)
            ) {
                if self.threads.failed.is_none() {
                    unsound("`main` ended before a thread it started");
                }
                self.switch(t);
            }
            if let Some(os) = self.threads.list[t].os.take() {
                os.join().unwrap_or_else(|p| std::panic::resume_unwind(p));
            }
        }
        match self.threads.failed.take() {
            Some(e) => Err(e),
            None => result,
        }
    }
}
