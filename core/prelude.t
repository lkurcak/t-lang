//! What's in scope in every module, after what the module declares and
//! imports itself.

export use crate::cmp::{Eq, Ord};
export use crate::iter::Iterator;
export use crate::mem::{Copy, Discard};
export use crate::num::{Int, Signed, Unsigned};
export use crate::num::{checked_add, checked_div, checked_mul, checked_neg, checked_rem, checked_sub};
export use crate::ops::{Fn, FnMut, FnOnce};
export use crate::option::Option;
export use crate::panic::{assert, assert_eq, assert_ne, panic};
export use crate::result::Result;
