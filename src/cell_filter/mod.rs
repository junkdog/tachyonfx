//! Cell filtering system for selective effect application.
//!
//! This module provides a comprehensive system for filtering terminal cells based on
//! various criteria, enabling effects to target specific subsets of cells rather than
//! applying to entire areas uniformly.

mod analyzer;
mod filter;
mod predicate;
mod processor;

pub(crate) use analyzer::*;
pub use filter::*;
pub use predicate::CellPredicate;
pub use processor::*;
