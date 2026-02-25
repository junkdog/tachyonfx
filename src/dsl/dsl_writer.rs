use compact_str::CompactString;

use crate::dsl::expressions::{Expr, FnCallInfo, Value};

/// A writer for formatting DSL expressions with smart formatting decisions.
///
/// The `DslWriter` provides a more maintainable and consistent approach to
/// formatting DSL expressions, with smarter handling of nested expressions,
/// method chains, and decisions about when to use single-line vs multi-line formatting.
pub(super) struct DslWriter {
    /// Current indentation level (in spaces)
    indent: usize,
    /// Indentation increment for each level (in spaces)
    indent_step: usize,
    /// Maximum target line length before preferring multi-line formatting
    max_line_length: usize,
    /// The formatted output
    output: CompactString,
    /// The current line's estimated length (used for formatting decisions)
    current_line_length: usize,
}

impl DslWriter {
    /// Creates a new `DslWriter` with default settings.
    pub(super) fn new() -> Self {
        Self {
            indent: 0,
            indent_step: 4,
            max_line_length: 80,
            output: CompactString::default(),
            current_line_length: 0,
        }
    }

    /// Formats an expression tree to a string.
    pub(super) fn format(expr: &Expr) -> CompactString {
        let mut writer = Self::new();
        writer.write_expr(expr);
        writer.output
    }

    /// Write an expression to the output.
    fn write_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Literal(value, _) => self.write_literal(value),
            Expr::Var { name, self_fns, .. } => self.write_var(name, self_fns),
            Expr::LetBinding { name, let_expr, .. } => self.write_let_binding(name, let_expr),
            Expr::ArrayRef(exprs, _) => self.write_array_ref(exprs),
            Expr::Array(exprs, _) => self.write_array(exprs),
            Expr::FnCall { call, self_fns, .. } => self.write_fn_call(call, self_fns),
            Expr::QualifiedMember { name, self_fns, .. } => self.write_var(name, self_fns),
            Expr::OptionSome(expr, _) => self.write_option_some(expr),
            Expr::Sequence { effects, self_fns, .. } => self.write_sequence(effects, self_fns),
            Expr::Parallel { effects, self_fns, .. } => self.write_parallel(effects, self_fns),
            Expr::StructInit { name, fields, .. } => self.write_struct_init(name, fields),
            Expr::Tuple(exprs, _) => self.write_tuple(exprs),
            Expr::Macro { name, args, .. } => self.write_macro(name, args),
            Expr::Delimiter { .. } => unreachable!("delimiter should be have been excluded"),
            Expr::SyntaxError { .. } => unreachable!("syntax errors should have been handled"),
        }
    }

    /// Write a literal value.
    fn write_literal(&mut self, value: &Value) {
        self.write(&value.format());
    }

    /// Write a variable with optional method chains.
    fn write_var(&mut self, name: &CompactString, self_fns: &[FnCallInfo]) {
        self.write(name);
        self.write_method_chains(self_fns);
    }

    /// Write a let binding expression.
    fn write_let_binding(&mut self, name: &CompactString, expr: &Expr) {
        self.write_indent();
        self.write("let ");
        self.write(name);
        self.write(" = ");

        // For complex expressions, put the value on the next line with indentation
        if self.is_complex_expr(expr) {
            self.increase_indent();
            self.new_line();
            self.write_expr(expr);
            self.decrease_indent();
        } else {
            self.write_expr(expr);
        }
    }

    /// Write an array reference expression.
    fn write_array_ref(&mut self, exprs: &[Expr]) {
        self.write("&");
        self.write_array(exprs);
    }

    /// Write an array expression.
    fn write_array(&mut self, exprs: &[Expr]) {
        self.write("[");

        if exprs.is_empty() {
            self.write("]");
            return;
        }

        if self.should_inline_exprs(exprs) {
            self.write_exprs_inline(exprs);
            self.write("]");
        } else {
            self.increase_indent();
            self.new_line();
            self.write_exprs_multiline(exprs);
            self.decrease_indent();
            self.new_line();
            self.write_indent();
            self.write("]");
        }
    }

    /// Write a function call with optional method chains.
    fn write_fn_call(&mut self, call: &FnCallInfo, self_fns: &[FnCallInfo]) {
        self.write(&call.name);
        self.write_args(&call.args);
        self.write_method_chains(self_fns);
    }

    /// Write a Some option wrapper.
    fn write_option_some(&mut self, expr: &Expr) {
        self.write("Some(");
        self.write_expr(expr);
        self.write(")");
    }

    /// Write a sequence of effects.
    fn write_sequence(&mut self, effects: &[Expr], self_fns: &[FnCallInfo]) {
        self.write("fx::sequence(&");
        self.write_array(effects);
        self.write(")");
        self.write_method_chains(self_fns);
    }

    /// Write a parallel set of effects.
    fn write_parallel(&mut self, effects: &[Expr], self_fns: &[FnCallInfo]) {
        self.write("fx::parallel(&");
        self.write_array(effects);
        self.write(")");
        self.write_method_chains(self_fns);
    }

    /// Write a struct initialization.
    fn write_struct_init(&mut self, name: &CompactString, fields: &[(CompactString, Expr)]) {
        self.write(name);
        self.write(" {");

        if fields.is_empty() {
            self.write("}");
            return;
        }

        self.increase_indent();
        self.new_line();

        for (i, (field_name, field_value)) in fields.iter().enumerate() {
            self.write_indent();
            self.write(field_name);
            self.write(": ");

            if self.is_complex_expr(field_value) {
                self.increase_indent();
                self.new_line();
                self.write_expr(field_value);
                self.decrease_indent();
            } else {
                self.write_expr(field_value);
            }

            if i < fields.len() - 1 {
                self.write(",");
                self.new_line();
            } else {
                self.write(",");
            }
        }

        self.decrease_indent();
        self.new_line();
        self.write_indent();
        self.write("}");
    }

    /// Write a tuple expression.
    fn write_tuple(&mut self, exprs: &[Expr]) {
        self.write("(");

        if exprs.is_empty() {
            self.write(")");
            return;
        }

        if self.should_inline_exprs(exprs) {
            self.write_exprs_inline(exprs);
            self.write(")");
        } else {
            self.increase_indent();
            self.new_line();
            self.write_exprs_multiline(exprs);
            self.decrease_indent();
            self.new_line();
            self.write_indent();
            self.write(")");
        }
    }

    /// Write a macro expression like vec![].
    fn write_macro(&mut self, name: &CompactString, args: &[Expr]) {
        self.write(name);
        self.write("![");

        if args.is_empty() {
            self.write("]");
            return;
        }

        if self.should_inline_exprs(args) {
            self.write_exprs_inline(args);
            self.write("]");
        } else {
            self.increase_indent();
            self.new_line();
            self.write_exprs_multiline(args);
            self.decrease_indent();
            self.new_line();
            self.write_indent();
            self.write("]");
        }
    }

    /// Write function arguments.
    fn write_args(&mut self, args: &[Expr]) {
        self.write("(");

        if args.is_empty() {
            self.write(")");
            return;
        }

        if self.should_inline_args(args) {
            self.write_exprs_inline(args);
            self.write(")");
        } else {
            self.increase_indent();
            self.new_line();
            self.write_exprs_multiline(args);
            self.decrease_indent();
            self.new_line();
            self.write_indent();
            self.write(")");
        }
    }

    /// Write method chains.
    fn write_method_chains(&mut self, self_fns: &[FnCallInfo]) {
        if self_fns.is_empty() {
            return;
        }

        // For simple no-arg methods, try to keep them on one line
        let simple_methods: Vec<_> = self_fns
            .iter()
            .filter(|f| f.args.is_empty())
            .collect();

        // If we only have simple methods and there aren't too many, inline them
        if !simple_methods.is_empty()
            && simple_methods.len() == self_fns.len()
            && simple_methods.len() <= 2
        {
            for method in simple_methods {
                self.write(".");
                self.write(&method.name);
                self.write("()");
            }
            return;
        }

        // Otherwise write one method per line with proper indentation
        self.increase_indent();
        for method in self_fns {
            self.new_line();
            self.write_indent();
            self.write(".");
            self.write(&method.name);
            self.write_args(&method.args);
        }
        self.decrease_indent();
    }

    /// Write multiple expressions inline with commas between them.
    fn write_exprs_inline(&mut self, exprs: &[Expr]) {
        for (i, expr) in exprs.iter().enumerate() {
            self.write_expr(expr);
            if i < exprs.len() - 1 {
                self.write(", ");
            }
        }
    }

    /// Write multiple expressions with one per line.
    fn write_exprs_multiline(&mut self, exprs: &[Expr]) {
        for (i, expr) in exprs.iter().enumerate() {
            self.write_indent();
            self.write_expr(expr);
            if i < exprs.len() - 1 {
                self.write(",");
                self.new_line();
            }
        }
    }

    fn increase_indent(&mut self) {
        self.indent += self.indent_step;
    }

    fn decrease_indent(&mut self) {
        self.indent -= self.indent_step;
    }

    /// Write the current indentation.
    fn write_indent(&mut self) {
        let spaces = " ".repeat(self.indent);
        self.write(&spaces);
    }

    /// Start a new line with proper indentation.
    fn new_line(&mut self) {
        self.write("\n");
        self.current_line_length = 0;
    }

    /// Write a string to the output.
    fn write(&mut self, s: &str) {
        self.output.push_str(s);
        self.current_line_length += s.len();
    }

    /// Determine if a list of expressions should be formatted inline.
    fn should_inline_exprs(&self, exprs: &[Expr]) -> bool {
        // If there are too many expressions, use multi-line
        if exprs.len() > 3 {
            return false;
        }

        // If any expression is complex, use multi-line
        if exprs.iter().any(|e| self.is_complex_expr(e)) {
            return false;
        }

        // Estimate the total length if inlined
        let estimated_length = exprs
            .iter()
            .map(|e| self.estimate_expr_length(e))
            .sum::<usize>()
            + (exprs.len() * 2); // account for ", " between expressions

        // If the total length is too long, use multi-line
        estimated_length <= self.max_line_length - self.indent
    }

    /// Determine if function arguments should be formatted inline.
    fn should_inline_args(&self, args: &[Expr]) -> bool {
        self.should_inline_exprs(args)
    }

    /// Determine if an expression is complex (should be formatted on multiple lines).
    fn is_complex_expr(&self, expr: &Expr) -> bool {
        self.is_complex_expr_impl(expr, 0)
    }

    #[allow(clippy::only_used_in_recursion)]
    fn is_complex_expr_impl(&self, expr: &Expr, depth: usize) -> bool {
        // Prevent infinite recursion
        if depth > 20 {
            return true;
        }

        match expr {
            // These types are always considered complex
            Expr::Sequence { .. } | Expr::Parallel { .. } | Expr::StructInit { .. } => true,

            // Function calls with many or complex arguments are complex
            Expr::FnCall { call, self_fns, .. } => {
                !self_fns.is_empty()
                    || call.args.len() > 2
                    || call
                        .args
                        .iter()
                        .any(|arg| self.is_complex_expr_impl(arg, depth + 1))
            },

            // Arrays with many or complex elements are complex
            Expr::Array(elements, _) | Expr::ArrayRef(elements, _) => {
                elements.len() > 3
                    || elements
                        .iter()
                        .any(|e| self.is_complex_expr_impl(e, depth + 1))
            },

            // Macros with many or complex arguments are complex
            Expr::Macro { args, .. } => {
                args.len() > 3
                    || args
                        .iter()
                        .any(|e| self.is_complex_expr_impl(e, depth + 1))
            },

            // Variables and qualified members with method chains are complex
            Expr::Var { self_fns, .. } | Expr::QualifiedMember { self_fns, .. } => {
                !self_fns.is_empty()
            },

            // Other expression types are generally simple
            _ => false,
        }
    }

    /// Estimate the length of an expression when formatted as a string.
    // todo: take current indentation into account + estimate shorter representations for
    // complex expressions
    fn estimate_expr_length(&self, expr: &Expr) -> usize {
        match expr {
            Expr::Literal(value, _) => value.format().len(),
            Expr::Var { name, self_fns, .. } => {
                let mut len = name.len();
                for method in self_fns {
                    len += method.name.len() + 2; // +2 for ".()"
                    len += method
                        .args
                        .iter()
                        .map(|arg| self.estimate_expr_length(arg))
                        .sum::<usize>();
                }
                len
            },
            Expr::QualifiedMember { name, self_fns, .. } => {
                let mut len = name.len();
                for method in self_fns {
                    len += method.name.len() + 2; // +2 for ".()"
                    len += method
                        .args
                        .iter()
                        .map(|arg| self.estimate_expr_length(arg))
                        .sum::<usize>();
                }
                len
            },
            Expr::OptionSome(inner, _) => 5 + self.estimate_expr_length(inner), /* "Some()" = 5 */
            // chars
            Expr::FnCall { call, self_fns, .. } => {
                let mut len = call.name.len() + 2; // +2 for "()"
                len += call
                    .args
                    .iter()
                    .map(|arg| self.estimate_expr_length(arg))
                    .sum::<usize>();
                for method in self_fns {
                    len += method.name.len() + 2; // +2 for ".()"
                    len += method
                        .args
                        .iter()
                        .map(|arg| self.estimate_expr_length(arg))
                        .sum::<usize>();
                }
                len
            },
            Expr::Macro { name, args, .. } => {
                let mut len = name.len() + 3; // +3 for "![]"
                len += args
                    .iter()
                    .map(|arg| self.estimate_expr_length(arg))
                    .sum::<usize>();
                if !args.is_empty() {
                    len += args.len() * 2 - 2; // For ", " between elements
                }
                len
            },
            // For complex expressions, just use a large value to encourage multi-line formatting
            _ => self.max_line_length,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsl::{
        expressions::{Expr, ExprSpan, Value},
        token_parsers::parse_ast,
        tokenizer::{sanitize_tokens, tokenize},
    };

    fn parse_expr(input: &str) -> Expr {
        tokenize(input)
            .map(sanitize_tokens)
            .and_then(parse_ast)
            .unwrap()
            .into_iter()
            .next()
            .unwrap()
    }

    #[test]
    fn test_simple_function_call() {
        let expr = parse_expr("fx::dissolve(500)");
        let formatted = DslWriter::format(&expr);
        assert_eq!(formatted, "fx::dissolve(500)");
    }

    #[test]
    fn test_function_call_with_method_chain() {
        let expr = parse_expr("fx::dissolve(500).filter(CellFilter::Text)");
        let formatted = DslWriter::format(&expr);
        assert_eq!(
            formatted,
            "fx::dissolve(500)\n    .filter(CellFilter::Text)"
        );
    }

    #[test]
    fn test_complex_function_call() {
        let expr = parse_expr(r#"fx::fade_to(Color::Red, Color::Blue, (500, CircOut))"#);
        let formatted = DslWriter::format(&expr);
        assert_eq!(
            formatted,
            "fx::fade_to(\n    Color::Red,\n    Color::Blue,\n    (500, Interpolation::CircOut)\n)"
        );
    }

    #[test]
    fn test_sequence() {
        let expr = parse_expr(
            r#"fx::sequence(&[fx::dissolve(500), fx::fade_to(Color::Red, Color::Blue, (1000, Linear))])"#,
        );
        let formatted = DslWriter::format(&expr);
        assert_eq!(
            formatted,
            "fx::sequence(&[\n    fx::dissolve(500),\n    fx::fade_to(\n        Color::Red,\n        Color::Blue,\n        (1000, Interpolation::Linear)\n    )\n])"
        );
    }

    #[test]
    fn test_let_binding() {
        let expr = parse_expr(r#"let color = Color::Red"#);
        let formatted = DslWriter::format(&expr);
        assert_eq!(formatted, "let color = Color::Red");
    }

    #[test]
    fn test_struct_init() {
        let expr = parse_expr(r#"Rect { x: 0, y: 0, width: 100, height: 100 }"#);
        let formatted = DslWriter::format(&expr);
        assert_eq!(
            formatted,
            "Rect {\n    x: 0,\n    y: 0,\n    width: 100,\n    height: 100,\n}"
        );
    }

    #[test]
    fn test_simple_methods_inline() {
        let expr = parse_expr(r#"fx::fade_to(Color::Red, 1000).clone().reversed()"#);
        let formatted = DslWriter::format(&expr);
        assert_eq!(
            formatted,
            "fx::fade_to(Color::Red, 1000).clone().reversed()"
        );
    }

    #[test]
    fn test_macro_formatting() {
        // Create a simple macro expression manually
        let span = ExprSpan::new(0, 0);
        let expr = Expr::Macro {
            name: "vec".into(),
            args: vec![
                Expr::Literal(Value::U32(1), span),
                Expr::Literal(Value::U32(2), span),
                Expr::Literal(Value::U32(3), span),
            ],
            span,
        };

        let formatted = DslWriter::format(&expr);
        assert_eq!(formatted, "vec![1, 2, 3]");
    }
}
