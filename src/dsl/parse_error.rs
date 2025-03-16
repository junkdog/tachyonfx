use std::fmt;
use crate::dsl::DslError;

/// Provides detailed information about errors that occurred while parsing or compiling
/// DSL expressions, including location information and context.
///
/// This error enhances the basic `DslError` with source code location information
/// and context, making it easier to identify and fix issues in DSL expressions.
#[derive(Debug)]
pub struct DslParseError {
    /// The underlying error that occurred during parsing
    pub source: DslError,
    /// The complete line of text where the error occurred
    context_line: String,
    /// The character range within the context line that caused the error
    error_range: std::ops::Range<usize>,
    /// The line number where the error occurred (1-based)
    line_number: u32,
}

impl DslParseError {
    pub(super) fn new(
        input: &str,
        cause: DslError,
    ) -> Self {
        let span = cause.span();

        if let Some(span) = span {
            let line_start_offset = input[0..span.start as usize]
                .rfind('\n')
                .map_or(0, |pos| pos + 1);
            let line_end_offset = input[span.start as usize..] // only consider current line
                .find('\n')
                .map_or(input.len(), |pos| span.end as usize + pos - 1);

            let context_line = input[line_start_offset..line_end_offset].to_string();
            let error_start = span.start as usize - line_start_offset;
            let error_end = span.end as usize - line_start_offset;
            let error_range = error_start..error_end;

            Self {
                source: cause,
                context_line,
                error_range,
                line_number: input[0..span.start as usize].lines().count() as u32,
            }
        } else {
            Self {
                source: cause,
                context_line: input.to_string(),
                error_range: 0..input.len(),
                line_number: 0,
            }
        }
    }

    /// Returns the portion of DSL that caused the error
    pub fn error_text(&self) -> &str {
        let range = self.error_range.clone();
        &self.context_line[range]
    }

    /// Returns the entire line of code that contained the error
    pub fn error_line(&self) -> &str {
        self.context_line.trim()
    }

    /// Returns the column position where the error starts (1-based)
    pub fn column(&self) -> u32 {
        self.error_range.start as u32 + 1
    }

    /// Returns the line number where the error occurred (1-based)
    pub fn line(&self) -> u32 {
        self.line_number
    }
}

impl fmt::Display for DslParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let location = format!("at line {} column {}", self.line(), self.column());

        writeln!(f, "Error in DSL expression {}: {}", location, self.source)?;

        let pointer_padding = " ".repeat(self.column() as usize - 1);
        if self.context_line.lines().count() == 1 {
            // If error spans multiple characters, underline the whole range
            let underline = "^".repeat(self.error_range.len().max(1));
            writeln!(f, "\n{}\n{pointer_padding}{underline}", self.context_line)
        } else {
            // For multi-line errors, just point to the start
            writeln!(f, "\n{}\n{pointer_padding}^", self.context_line)
        }
    }
}