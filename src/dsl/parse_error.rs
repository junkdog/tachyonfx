use core::fmt;
use std::error;

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
    /// The complete input text where the error occurred
    input: String,
    /// Line and column information
    location: Location,
}

#[derive(Debug, Clone)]
struct Location {
    /// The line number where the error starts (1-based)
    start_line: usize,
    /// The column number where the error starts (1-based)
    start_column: usize,
    /// The line number where the error ends (1-based)
    end_line: usize,
    /// The column number where the error ends (1-based)
    end_column: usize,
}

impl DslParseError {
    pub(super) fn new(input: &str, cause: DslError) -> Self {
        let location = if let Some(span) = cause.span() {
            // Calculate line and column information
            let mut start_line = 1;
            let mut start_column = 1;
            let mut end_line = 1;
            let mut end_column = 1;

            for (i, c) in input.char_indices() {
                if i >= span.start as usize {
                    break;
                }
                if c == '\n' {
                    start_line += 1;
                    start_column = 1;
                } else {
                    start_column += 1;
                }
            }

            // Reset for end position calculation
            for (i, c) in input.char_indices() {
                if i >= span.end as usize {
                    break;
                }
                if c == '\n' {
                    end_line += 1;
                    end_column = 1;
                } else {
                    end_column += 1;
                }
            }

            Location { start_line, start_column, end_line, end_column }
        } else {
            // Default location if no span is available
            Location {
                start_line: 1,
                start_column: 1,
                end_line: 1,
                end_column: 1,
            }
        };

        Self { source: cause, input: input.to_string(), location }
    }

    /// Returns the line where the error starts
    pub fn start_line(&self) -> usize {
        self.location.start_line
    }

    /// Returns the column where the error starts
    pub fn start_column(&self) -> usize {
        self.location.start_column
    }

    /// Returns the line where the error ends
    pub fn end_line(&self) -> usize {
        self.location.end_line
    }

    /// Returns the column where the error ends
    pub fn end_column(&self) -> usize {
        self.location.end_column
    }

    /// Returns the entire context around the error, including nearby lines
    pub fn context(&self) -> String {
        let context_lines = 2; // Number of lines before and after the error to show
        let lines: Vec<&str> = self.input.lines().collect();

        let start_idx = self
            .location
            .start_line
            .saturating_sub(context_lines + 1);
        let end_idx = (self.location.end_line + context_lines).min(lines.len());

        let mut result = String::new();

        for (i, line) in lines[start_idx..end_idx].iter().enumerate() {
            let line_num = start_idx + i + 1;
            let line_indicator =
                if line_num >= self.location.start_line && line_num <= self.location.end_line {
                    ">"
                } else {
                    " "
                };

            result.push_str(&format!("{line_indicator:>2} {line_num} | {line}\n"));

            // Add underline for error location
            if line_num >= self.location.start_line && line_num <= self.location.end_line {
                let start_col = if line_num == self.location.start_line {
                    self.location.start_column
                } else {
                    1
                };
                let end_col = if line_num == self.location.end_line {
                    self.location.end_column
                } else {
                    line.len() + 1
                };

                let padding = " ".repeat(7);
                let leading_space = " ".repeat(start_col.saturating_sub(1));
                let underline = "^".repeat((end_col - start_col).max(1));

                result.push_str(&format!("{padding}{leading_space}{underline}\n"));
            }
        }

        result
    }

    /// Returns the portion of text that caused the error
    pub fn error_text(&self) -> String {
        let lines: Vec<&str> = self.input.lines().collect();

        if self.location.start_line == self.location.end_line {
            // Single line error
            let line = lines
                .get(self.location.start_line - 1)
                .unwrap_or(&"");
            let start_col = self.location.start_column.saturating_sub(1);
            let end_col = self.location.end_column.min(line.len() + 1);

            line[start_col..end_col].to_string()
        } else {
            // Multi-line error
            let mut result = String::new();

            for line_num in self.location.start_line..=self.location.end_line {
                if line_num > self.location.start_line {
                    result.push('\n');
                }

                if let Some(line) = lines.get(line_num - 1) {
                    let start_col = if line_num == self.location.start_line {
                        self.location.start_column.saturating_sub(1)
                    } else {
                        0
                    };

                    let end_col = if line_num == self.location.end_line {
                        self.location.end_column.min(line.len() + 1)
                    } else {
                        line.len()
                    };

                    result.push_str(&line[start_col..end_col]);
                }
            }

            result
        }
    }
}

impl fmt::Display for DslParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.location.start_line == self.location.end_line {
            writeln!(
                f,
                "Error at line {}:{} to {}:{}: {}",
                self.location.start_line,
                self.location.start_column,
                self.location.end_line,
                self.location.end_column,
                self.source
            )?;
        } else {
            writeln!(
                f,
                "Error from line {}:{} to line {}:{}: {}",
                self.location.start_line,
                self.location.start_column,
                self.location.end_line,
                self.location.end_column,
                self.source
            )?;
        }

        write!(f, "{}", self.context())
    }
}

impl error::Error for DslParseError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        Some(&self.source)
    }
}
