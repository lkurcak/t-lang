//! The intrinsics: the functions and types the built-in libraries declare
//! with `#[intrinsic]` and no body or fields, which the compiler provides.
//! They're the trusted base: each is checked like any function, by its
//! signature, and what it does is what the checker trusts (see
//! docs/46-unsafe-cell.md, I1).
//!
//! Each is named here once, by its symbol, and resolved once, when the
//! checker reads its declaration (`Intrinsic::from_symbol`): what isn't
//! here is an error there. Monomorphisation hands each copy of one on with
//! which it is (`Executable::intrinsic`), and the interpreter and the C
//! backend `match` on that, so one that either doesn't run is a compile
//! error of the compiler's, not a crash of a program's.

/// An intrinsic function, or a method of an intrinsic or built-in type.
/// A family of methods is one variant, with the method as its operand, so
/// what runs the family can `match` on all of them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Intrinsic {
    /// `core::mem::swap`.
    Swap,
    /// `core::cell::Unsafe::as_mut`: what's in it, as `&mut`.
    UnsafeAsMut,
    /// `core::cell::Unsafe::as_ref`: what's in it, as `&`.
    UnsafeAsRef,
    /// `core::sync::spin`.
    Spin,
    /// A method of `core::sync::Atomic`.
    Atomic(AtomicOp),
    /// A method of `core::volatile::Volatile`.
    Volatile(VolatileOp),
    /// `core::panic::panic`.
    Panic,
    /// A method of `core::panic::Message`.
    Message(MessageOp),
    /// `core::num::checked_mul`.
    CheckedMul,
    /// `core::num::wrapping_mul`.
    WrappingMul,
    /// `core::num::wrap`.
    Wrap,
    /// `core::num::least`.
    Least,
    /// `core::num::greatest`.
    Greatest,
    /// `core::slice::empty`, and `empty_mut`: a slice of nothing.
    EmptySlice { mutable: bool },
    /// `core::str::from_utf8_unchecked`.
    FromUtf8Unchecked,
    /// `str::as_bytes`.
    StrAsBytes,
    /// `str::len`.
    StrLen,
    /// A method of a slice, `[T]`.
    Slice(SliceOp),
    /// A method of an array, `[T; N]`.
    Array(ArrayOp),
    /// `core::array::from_fn_mut`.
    ArrayFromFn,
    /// A method of `alloc::raw::RawBuf`.
    RawBuf(RawBufOp),
    /// `alloc::raw::out_of_memory`.
    OutOfMemory,
    /// A method of `alloc::arena::Slots`, where an arena keeps its values.
    Slots(SlotsOp),
    /// An intrinsic method of `std::io::Stdin`.
    Stdin(StdinOp),
    /// An intrinsic method of `std::io::Stdout` or `Stderr`.
    Output(Output, OutputOp),
    /// A method of `std::env::Args`.
    Args(ArgsOp),
    /// A method of `std::env::Env`.
    Env(EnvOp),
    /// `std::thread::start`.
    ThreadStart,
    /// `std::thread::JoinHandle::join`.
    ThreadJoin,
}

/// A method of `core::sync::Atomic`, each one sequentially consistent step.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AtomicOp {
    Load,
    Swap,
    CompareSwap,
    FetchAnd,
    FetchOr,
    FetchXor,
    FetchAdd,
    FetchSub,
}

/// A method of `core::volatile::Volatile`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VolatileOp {
    At,
    Read,
    Write,
}

/// A method of `core::panic::Message`, the message `panic_with` writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MessageOp {
    New,
    PushStr,
    PushChar,
    Abort,
}

/// A method of a slice.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SliceOp {
    Len,
    Index,
    IndexMut,
    Get,
    GetMut,
    SplitAt,
    SplitAtMut,
}

/// A method of an array.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ArrayOp {
    Len,
    AsSlice,
    AsMutSlice,
}

/// A method of `alloc::raw::RawBuf`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RawBufOp {
    New,
    Empty,
    Cap,
    Grow,
    Write,
    Read,
    Slice,
    SliceMut,
    Free,
}

/// A method of `alloc::arena::Slots`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SlotsOp {
    New,
    Alloc,
    Len,
}

/// An intrinsic method of `std::io::Stdin`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StdinOp {
    Read,
    IsTerminal,
}

/// One of `std::io`'s streams that are written: `Stdout` or `Stderr`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Output {
    Stdout,
    Stderr,
}

/// An intrinsic method of `std::io::Stdout` or `Stderr`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OutputOp {
    Write,
    Flush,
    IsTerminal,
}

/// A method of `std::env::Args`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ArgsOp {
    Program,
    Len,
    Index,
}

/// A method of `std::env::Env`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EnvOp {
    Len,
    Index,
}

impl Intrinsic {
    /// Every intrinsic, in the order the libraries declare them.
    pub const ALL: &[Intrinsic] = {
        use Intrinsic::*;
        &[
            Swap,
            UnsafeAsMut,
            UnsafeAsRef,
            Spin,
            Atomic(AtomicOp::Load),
            Atomic(AtomicOp::Swap),
            Atomic(AtomicOp::CompareSwap),
            Atomic(AtomicOp::FetchAnd),
            Atomic(AtomicOp::FetchOr),
            Atomic(AtomicOp::FetchXor),
            Atomic(AtomicOp::FetchAdd),
            Atomic(AtomicOp::FetchSub),
            Volatile(VolatileOp::At),
            Volatile(VolatileOp::Read),
            Volatile(VolatileOp::Write),
            Panic,
            Message(MessageOp::New),
            Message(MessageOp::PushStr),
            Message(MessageOp::PushChar),
            Message(MessageOp::Abort),
            CheckedMul,
            WrappingMul,
            Wrap,
            Least,
            Greatest,
            EmptySlice { mutable: false },
            EmptySlice { mutable: true },
            FromUtf8Unchecked,
            StrAsBytes,
            StrLen,
            Slice(SliceOp::Len),
            Slice(SliceOp::Index),
            Slice(SliceOp::IndexMut),
            Slice(SliceOp::Get),
            Slice(SliceOp::GetMut),
            Slice(SliceOp::SplitAt),
            Slice(SliceOp::SplitAtMut),
            Array(ArrayOp::Len),
            Array(ArrayOp::AsSlice),
            Array(ArrayOp::AsMutSlice),
            ArrayFromFn,
            RawBuf(RawBufOp::New),
            RawBuf(RawBufOp::Empty),
            RawBuf(RawBufOp::Cap),
            RawBuf(RawBufOp::Grow),
            RawBuf(RawBufOp::Write),
            RawBuf(RawBufOp::Read),
            RawBuf(RawBufOp::Slice),
            RawBuf(RawBufOp::SliceMut),
            RawBuf(RawBufOp::Free),
            OutOfMemory,
            Slots(SlotsOp::New),
            Slots(SlotsOp::Alloc),
            Slots(SlotsOp::Len),
            Stdin(StdinOp::Read),
            Stdin(StdinOp::IsTerminal),
            Output(self::Output::Stdout, OutputOp::Write),
            Output(self::Output::Stdout, OutputOp::Flush),
            Output(self::Output::Stdout, OutputOp::IsTerminal),
            Output(self::Output::Stderr, OutputOp::Write),
            Output(self::Output::Stderr, OutputOp::Flush),
            Output(self::Output::Stderr, OutputOp::IsTerminal),
            Args(ArgsOp::Program),
            Args(ArgsOp::Len),
            Args(ArgsOp::Index),
            Env(EnvOp::Len),
            Env(EnvOp::Index),
            ThreadStart,
            ThreadJoin,
        ]
    };

    /// Its symbol, or for a method, its type's symbol and its name (`[T]`
    /// for a slice's, `[T; N]` for an array's, `str` for a string's), as
    /// the checker names its declaration.
    pub fn symbol(self) -> &'static str {
        use Intrinsic::*;
        match self {
            Swap => "core::mem::swap",
            UnsafeAsMut => "core::cell::Unsafe::as_mut",
            UnsafeAsRef => "core::cell::Unsafe::as_ref",
            Spin => "core::sync::spin",
            Atomic(op) => match op {
                AtomicOp::Load => "core::sync::Atomic::load",
                AtomicOp::Swap => "core::sync::Atomic::swap",
                AtomicOp::CompareSwap => "core::sync::Atomic::compare_swap",
                AtomicOp::FetchAnd => "core::sync::Atomic::fetch_and",
                AtomicOp::FetchOr => "core::sync::Atomic::fetch_or",
                AtomicOp::FetchXor => "core::sync::Atomic::fetch_xor",
                AtomicOp::FetchAdd => "core::sync::Atomic::fetch_add",
                AtomicOp::FetchSub => "core::sync::Atomic::fetch_sub",
            },
            Volatile(op) => match op {
                VolatileOp::At => "core::volatile::Volatile::at",
                VolatileOp::Read => "core::volatile::Volatile::read",
                VolatileOp::Write => "core::volatile::Volatile::write",
            },
            Panic => "core::panic::panic",
            Message(op) => match op {
                MessageOp::New => "core::panic::Message::new",
                MessageOp::PushStr => "core::panic::Message::push_str",
                MessageOp::PushChar => "core::panic::Message::push_char",
                MessageOp::Abort => "core::panic::Message::abort",
            },
            CheckedMul => "core::num::checked_mul",
            WrappingMul => "core::num::wrapping_mul",
            Wrap => "core::num::wrap",
            Least => "core::num::least",
            Greatest => "core::num::greatest",
            EmptySlice { mutable: false } => "core::slice::empty",
            EmptySlice { mutable: true } => "core::slice::empty_mut",
            FromUtf8Unchecked => "core::str::from_utf8_unchecked",
            StrAsBytes => "str::as_bytes",
            StrLen => "str::len",
            Slice(op) => match op {
                SliceOp::Len => "[T]::len",
                SliceOp::Index => "[T]::index",
                SliceOp::IndexMut => "[T]::index_mut",
                SliceOp::Get => "[T]::get",
                SliceOp::GetMut => "[T]::get_mut",
                SliceOp::SplitAt => "[T]::split_at",
                SliceOp::SplitAtMut => "[T]::split_at_mut",
            },
            Array(op) => match op {
                ArrayOp::Len => "[T; N]::len",
                ArrayOp::AsSlice => "[T; N]::as_slice",
                ArrayOp::AsMutSlice => "[T; N]::as_mut_slice",
            },
            ArrayFromFn => "core::array::from_fn_mut",
            RawBuf(op) => match op {
                RawBufOp::New => "alloc::raw::RawBuf::new",
                RawBufOp::Empty => "alloc::raw::RawBuf::empty",
                RawBufOp::Cap => "alloc::raw::RawBuf::cap",
                RawBufOp::Grow => "alloc::raw::RawBuf::grow",
                RawBufOp::Write => "alloc::raw::RawBuf::write",
                RawBufOp::Read => "alloc::raw::RawBuf::read",
                RawBufOp::Slice => "alloc::raw::RawBuf::slice",
                RawBufOp::SliceMut => "alloc::raw::RawBuf::slice_mut",
                RawBufOp::Free => "alloc::raw::RawBuf::free",
            },
            OutOfMemory => "alloc::raw::out_of_memory",
            Slots(op) => match op {
                SlotsOp::New => "alloc::arena::Slots::new",
                SlotsOp::Alloc => "alloc::arena::Slots::alloc",
                SlotsOp::Len => "alloc::arena::Slots::len",
            },
            Stdin(op) => match op {
                StdinOp::Read => "std::io::Stdin::os_read",
                StdinOp::IsTerminal => "std::io::Stdin::os_is_terminal",
            },
            Output(self::Output::Stdout, op) => match op {
                OutputOp::Write => "std::io::Stdout::os_write",
                OutputOp::Flush => "std::io::Stdout::os_flush",
                OutputOp::IsTerminal => "std::io::Stdout::os_is_terminal",
            },
            Output(self::Output::Stderr, op) => match op {
                OutputOp::Write => "std::io::Stderr::os_write",
                OutputOp::Flush => "std::io::Stderr::os_flush",
                OutputOp::IsTerminal => "std::io::Stderr::os_is_terminal",
            },
            Args(op) => match op {
                ArgsOp::Program => "std::env::Args::program",
                ArgsOp::Len => "std::env::Args::len",
                ArgsOp::Index => "std::env::Args::index",
            },
            Env(op) => match op {
                EnvOp::Len => "std::env::Env::len",
                EnvOp::Index => "std::env::Env::index",
            },
            ThreadStart => "std::thread::start",
            ThreadJoin => "std::thread::JoinHandle::join",
        }
    }

    /// Its name, without its path: a method's, `swap` for `Atomic::swap`.
    pub fn name(self) -> &'static str {
        let s = self.symbol();
        s.rsplit("::").next().unwrap_or(s)
    }

    /// The intrinsic of symbol `symbol`, if there's one.
    pub fn from_symbol(symbol: &str) -> Option<Intrinsic> {
        Self::ALL.iter().copied().find(|i| i.symbol() == symbol)
    }
}

/// An intrinsic type: a struct a built-in library declares with no fields,
/// which only the compiler makes and looks inside, by the intrinsic methods
/// of its impls. Each holds values of its type arguments, if it has any,
/// as a box does: it's covariant in them, frozen if they are, and dropping
/// it drops them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IntrinsicType {
    /// `core::panic::Message`.
    Message,
    /// `alloc::boxed::Box`, which is `Type::Box`.
    Box,
    /// `alloc::raw::RawBuf`.
    RawBuf,
    /// `alloc::arena::Slots`.
    Slots,
    /// `std::io::Stdin`.
    Stdin,
    /// `std::io::Stdout`.
    Stdout,
    /// `std::io::Stderr`.
    Stderr,
    /// `std::env::Args`.
    Args,
    /// `std::env::Env`.
    Env,
    /// `std::thread::Threads`.
    Threads,
    /// `std::thread::JoinHandle`.
    JoinHandle,
}

impl IntrinsicType {
    /// Every intrinsic type.
    pub const ALL: &[IntrinsicType] = {
        use IntrinsicType::*;
        &[
            Message, Box, RawBuf, Slots, Stdin, Stdout, Stderr, Args, Env, Threads, JoinHandle,
        ]
    };

    /// Its symbol.
    pub fn symbol(self) -> &'static str {
        use IntrinsicType::*;
        match self {
            Message => "core::panic::Message",
            Box => crate::check::BOX,
            RawBuf => "alloc::raw::RawBuf",
            Slots => crate::check::ARENA_SLOTS,
            Stdin => "std::io::Stdin",
            Stdout => "std::io::Stdout",
            Stderr => "std::io::Stderr",
            Args => "std::env::Args",
            Env => "std::env::Env",
            Threads => "std::thread::Threads",
            JoinHandle => "std::thread::JoinHandle",
        }
    }

    /// The intrinsic type of symbol `symbol`, if there's one.
    pub fn from_symbol(symbol: &str) -> Option<IntrinsicType> {
        Self::ALL.iter().copied().find(|t| t.symbol() == symbol)
    }
}

/// Is the struct of symbol `symbol` an intrinsic type?
pub fn is_intrinsic_type(symbol: &str) -> bool {
    IntrinsicType::from_symbol(symbol).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each symbol names one intrinsic, and back.
    #[test]
    fn symbols_name_one_each() {
        for &i in Intrinsic::ALL {
            assert_eq!(Intrinsic::from_symbol(i.symbol()), Some(i), "{i:?}");
        }
        for &t in IntrinsicType::ALL {
            assert_eq!(IntrinsicType::from_symbol(t.symbol()), Some(t), "{t:?}");
        }
    }
}
