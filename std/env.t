//! What the program was started with: its arguments, and the variables
//! of its environment. Both are in the `Os` that `main` is given, and are
//! `Copy`: they can be read, never changed, so it doesn't matter who reads
//! them when (see docs/47-io.md).
//!
//! Each is UTF-8: a program started with an argument or a variable that
//! isn't says so and stops, before `main`. Each lives as long as the
//! program, so it's a `&'static str`.

use core::iter::Iterator;

/// The program's arguments: the words after its name on the command
/// line, `a` and `b` in `prog a b`.
#[derive(Discard, Copy, Send, Sync)]
#[intrinsic]
export struct Args {}

impl Args {
    /// What the program was started as: its name, or path, as the
    /// command line gave it. Not an argument, so not among them.
    #[intrinsic]
    export fn program(&self) -> &'static str;

    /// How many arguments there are.
    #[intrinsic]
    #[measure]
    export fn len(&self) -> usize;

    /// Are there none?
    #[measure]
    export fn is_empty(&self) -> bool { self.len() == 0 }

    /// Argument `i`, from 0, which is less than `len()`.
    #[intrinsic]
    export fn index(&self, i: { i: usize | i < self.len() }) -> &'static str;

    /// Argument `i`, from 0, or `None` if there are only `i` or fewer.
    export fn get(&self, i: usize) -> Option<&'static str> {
        if i < self.len() { Some(self.index(i)) } else { None }
    }

    /// The arguments, in order.
    export fn iter(&self) -> ArgsIter { ArgsIter { args: *self, next: 0 } }
}

/// The arguments, in order: what `Args::iter` gives.
#[derive(Discard, Copy, Send, Sync)]
export struct ArgsIter { args: Args, next: usize }

impl Iterator for ArgsIter {
    type Item = &'static str;
    effect e = {};

    fn next(&mut self) -> Option<&'static str> {
        let i = self.next;
        if i < self.args.len() {
            self.next = i + 1;
            Some(self.args.index(i))
        } else {
            None
        }
    }
}

/// The variables of the program's environment: names, each with a value,
/// such as `HOME` and `/home/ada`.
#[derive(Discard, Copy, Send, Sync)]
#[intrinsic]
export struct Env {}

/// A variable of the environment: its name, and its value.
#[derive(Discard, Copy, Debug, Eq, Send, Sync)]
export struct Var { export name: &'static str, export value: &'static str }

impl Env {
    /// The value of the variable named `name`, or `None` if there's none.
    export fn get(&self, name: &str) -> Option<&'static str> {
        for v in self.vars() {
            if v.name == name {
                return Some(v.value);
            }
        }
        None
    }

    /// Every variable, in the order the environment has them.
    export fn vars(&self) -> Vars { Vars { env: *self, next: 0 } }

    /// How many variables there are.
    #[intrinsic]
    #[measure]
    fn len(&self) -> usize;

    /// Variable `i`, from 0, which is less than `len()`.
    #[intrinsic]
    fn index(&self, i: { i: usize | i < self.len() }) -> Var;
}

/// Every variable of the environment, in order: what `Env::vars` gives.
#[derive(Discard, Copy, Send, Sync)]
export struct Vars { env: Env, next: usize }

impl Iterator for Vars {
    type Item = Var;
    effect e = {};

    fn next(&mut self) -> Option<Var> {
        let i = self.next;
        if i < self.env.len() {
            self.next = i + 1;
            Some(self.env.index(i))
        } else {
            None
        }
    }
}
