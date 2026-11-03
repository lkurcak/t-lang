//! What a program is built for (docs/39-freestanding.md).

/// A machine with an operating system and a C library, or one with
/// neither, whose platform gives the program a few functions: where to
/// write a panic's message, how to stop, and, for `alloc`, memory.
/// Checking doesn't depend on it, but for which libraries a program has:
/// a target provides some, and a program can use only those.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Target {
    #[default]
    Hosted,
    Freestanding,
}

impl Target {
    pub const ALL: [Target; 2] = [Target::Hosted, Target::Freestanding];

    pub fn name(self) -> &'static str {
        match self {
            Target::Hosted => "hosted",
            Target::Freestanding => "freestanding",
        }
    }

    pub fn named(name: &str) -> Option<Target> {
        Target::ALL.into_iter().find(|t| t.name() == name)
    }

    /// The libraries built in that it provides, beside core: what a
    /// package with no `t.toml` has.
    pub fn libraries(self) -> &'static [&'static str] {
        match self {
            Target::Hosted => &["alloc", "std"],
            Target::Freestanding => &["alloc"],
        }
    }

    /// Does it provide library `name`?
    pub fn has(self, name: &str) -> bool {
        name == "core" || self.libraries().contains(&name)
    }

    /// What's wrong with using library `name`, which it doesn't provide.
    pub(crate) fn lacks(self, name: &str) -> String {
        format!(
            "`{name}` is for the hosted target, and this is built for the {} one",
            self.name()
        )
    }
}
