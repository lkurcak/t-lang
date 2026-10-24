//! What's in scope in every module, after what the module declares and
//! imports itself.

pub use crate::cmp::Ord;
pub use crate::iter::Iterator;
pub use crate::num::{checked_add, checked_div, checked_mul, checked_neg, checked_rem, checked_sub};
pub use crate::option::Option;
pub use crate::panic::panic;
pub use crate::result::Result;
