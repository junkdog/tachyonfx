//! Code completion support for the tachyonfx DSL.
//!
//! This module provides intelligent(?), context-aware code completion for DSL
//! expressions. The completion engine analyzes source code structure, tracks variable
//! bindings, and suggests appropriate completions based on cursor position.
//!
//! # Features
//!
//! - **Context-aware suggestions**: Understands namespaces, method chains, function
//!   calls, and struct initialization
//! - **Fuzzy matching**: Filters completions using prefix, acronym, and subsequence
//!   matching
//! - **Type tracking**: Tracks `let` bindings and suggests variables where
//!   type-appropriate
//! - **Parameter hints**: Shows expected parameter types in function calls
//!
//! # Quick Start
//!
//! ```
//! use tachyonfx::dsl::CompletionEngine;
//!
//! let engine = CompletionEngine::new();
//!
//! // Get completions at cursor position
//! let source = "fx::fade";
//! let cursor_pos = source.len() as u32;
//! let completions = engine.completions(source, cursor_pos);
//!
//! // Filter to effects starting with "fade"
//! for item in completions {
//!     println!("{}: {}", item.label, item.detail);
//! }
//! ```

mod context;
mod dsl_type;
mod engine;
mod macros;
mod matcher;
mod types;

pub use engine::CompletionEngine;
pub use types::{CompletionItem, CompletionKind};
