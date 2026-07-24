/**
 * Diagnostic rendering with colorized output and source context.
 *
 * @author Ruyi Team
 * @date 2026-05-02
 */
use crate::diagnostics::codes::ErrorCode;
use std::env;
use std::io::{self, Write};

/// Source code location for diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLocation {
    pub file: Option<String>,
    pub line: usize,
    pub column: usize,
    pub end_line: Option<usize>,
    pub end_column: Option<usize>,
}

impl SourceLocation {
    pub fn new(line: usize, column: usize) -> Self {
        Self {
            file: None,
            line,
            column,
            end_line: None,
            end_column: None,
        }
    }

    pub fn with_file(file: &str, line: usize, column: usize) -> Self {
        Self {
            file: Some(file.to_string()),
            line,
            column,
            end_line: None,
            end_column: None,
        }
    }

    pub fn span(line: usize, col_start: usize, line_end: usize, col_end: usize) -> Self {
        Self {
            file: None,
            line,
            column: col_start,
            end_line: Some(line_end),
            end_column: Some(col_end),
        }
    }

    pub fn format_location(&self) -> String {
        if let Some(ref file) = self.file {
            format!("{}:{}:{}", file, self.line, self.column)
        } else {
            format!("{}:{}", self.line, self.column)
        }
    }
}

/// Severity level for rendered diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderSeverity {
    Error,
    Warning,
    Note,
    Help,
}

impl RenderSeverity {}

/// A renderable diagnostic with optional source code context.
#[derive(Debug, Clone)]
pub struct RenderDiagnostic {
    pub code: Option<ErrorCode>,
    pub severity: RenderSeverity,
    pub message: String,
    pub location: Option<SourceLocation>,
    pub source_context: Option<SourceContext>,
    pub children: Vec<RenderDiagnostic>,
    pub suggestions: Vec<String>,
}

impl RenderDiagnostic {
    pub fn error(message: &str) -> Self {
        Self {
            code: None,
            severity: RenderSeverity::Error,
            message: message.to_string(),
            location: None,
            source_context: None,
            children: Vec::new(),
            suggestions: Vec::new(),
        }
    }

    pub fn with_code(mut self, code: ErrorCode) -> Self {
        self.code = Some(code);
        self
    }

    pub fn at(mut self, location: SourceLocation) -> Self {
        self.location = Some(location);
        self
    }

    pub fn with_context(mut self, context: SourceContext) -> Self {
        self.source_context = Some(context);
        self
    }

    pub fn with_child(mut self, child: RenderDiagnostic) -> Self {
        self.children.push(child);
        self
    }

    pub fn with_suggestion(mut self, suggestion: &str) -> Self {
        self.suggestions.push(suggestion.to_string());
        self
    }
}

/// Source code context for error display.
#[derive(Debug, Clone)]
pub struct SourceContext {
    pub file: String,
    pub source_lines: Vec<String>,
    pub highlight_range: Option<(usize, usize)>,
}

impl SourceContext {
    pub fn single_line(file: &str, line: &str, highlight: (usize, usize)) -> Self {
        Self {
            file: file.to_string(),
            source_lines: vec![line.to_string()],
            highlight_range: Some(highlight),
        }
    }

    pub fn multi_line(
        file: &str,
        lines: &[&str],
        _primary_line: usize,
        highlight: (usize, usize),
    ) -> Self {
        Self {
            file: file.to_string(),
            source_lines: lines.iter().map(|s| s.to_string()).collect(),
            highlight_range: Some(highlight),
        }
    }
}

/// Terminal color support.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorScheme {
    Auto,
    Always,
    Never,
}

impl ColorScheme {
    pub fn supports_colors(&self) -> bool {
        match self {
            ColorScheme::Auto => !env::var("TERM").map(|t| t == "dumb").unwrap_or(false),
            ColorScheme::Always => true,
            ColorScheme::Never => false,
        }
    }
}

/// ANSI color codes for terminal output.
#[derive(Debug, Clone, Copy)]
pub enum Color {
    Red,
    Green,
    Yellow,
    Blue,
    Bold,
    Underline,
    Reset,
}

impl Color {
    pub fn code(&self, colored: bool) -> &'static str {
        if !colored {
            return "";
        }
        match self {
            Color::Red => "\x1b[31m",
            Color::Green => "\x1b[32m",
            Color::Yellow => "\x1b[33m",
            Color::Blue => "\x1b[34m",
            Color::Bold => "\x1b[1m",
            Color::Underline => "\x1b[4m",
            Color::Reset => "\x1b[0m",
        }
    }
}

/// Console formatting utilities.
pub struct ConsoleFormatter {
    colored: bool,
}

impl ConsoleFormatter {
    pub fn new(scheme: ColorScheme) -> Self {
        Self {
            colored: scheme.supports_colors(),
        }
    }

    pub fn colorize(&self, text: &str, color: Color) -> String {
        if self.colored {
            format!("{}{}{}", color.code(true), text, Color::Reset.code(true))
        } else {
            text.to_string()
        }
    }

    pub fn error_color(&self, text: &str) -> String {
        self.colorize(text, Color::Red)
    }

    pub fn warning_color(&self, text: &str) -> String {
        self.colorize(text, Color::Yellow)
    }

    pub fn note_color(&self, text: &str) -> String {
        self.colorize(text, Color::Blue)
    }

    pub fn help_color(&self, text: &str) -> String {
        self.colorize(text, Color::Green)
    }

    pub fn bold(&self, text: &str) -> String {
        self.colorize(text, Color::Bold)
    }

    pub fn underline(&self, text: &str) -> String {
        self.colorize(text, Color::Underline)
    }

    pub fn file_location(&self, location: &str) -> String {
        if self.colored {
            format!("\x1b[34m{}\x1b[0m", location)
        } else {
            location.to_string()
        }
    }

    pub fn underline_span(&self, line: &str, start: usize, end: usize) -> String {
        if !self.colored {
            return line.to_string();
        }
        let (before, highlight, after) = Self::split_at_span(line, start, end);
        format!("{}\x1b[31m\x1b[4m{}\x1b[0m{}", before, highlight, after)
    }

    fn split_at_span(line: &str, start: usize, end: usize) -> (&str, &str, &str) {
        let chars: Vec<char> = line.chars().collect();
        let start_idx = start.min(chars.len());
        let end_idx = end.min(chars.len() + 1);

        let before: String = chars[..start_idx].iter().collect();
        let highlight: String = chars[start_idx..end_idx].iter().collect();
        let after: String = chars[end_idx..].iter().collect();
        (
            Box::leak(before.into_boxed_str()),
            Box::leak(highlight.into_boxed_str()),
            Box::leak(after.into_boxed_str()),
        )
    }
}

/// Main diagnostic renderer.
pub struct DiagnosticRenderer<W: Write> {
    writer: W,
    formatter: ConsoleFormatter,
    show_sources: bool,
    show_code: bool,
    show_children: bool,
}

impl<W: Write> DiagnosticRenderer<W> {
    pub fn new(writer: W, color_scheme: ColorScheme) -> Self {
        Self {
            writer,
            formatter: ConsoleFormatter::new(color_scheme),
            show_sources: true,
            show_code: true,
            show_children: true,
        }
    }

    pub fn with_sources(mut self, show: bool) -> Self {
        self.show_sources = show;
        self
    }

    pub fn with_code(mut self, show: bool) -> Self {
        self.show_code = show;
        self
    }

    pub fn with_children(mut self, show: bool) -> Self {
        self.show_children = show;
        self
    }

    pub fn render(&mut self, diagnostic: &RenderDiagnostic) -> io::Result<()> {
        self.render_diagnostic(diagnostic, 0)
    }

    fn render_diagnostic(&mut self, diag: &RenderDiagnostic, depth: usize) -> io::Result<()> {
        let prefix = if depth > 0 { "  " } else { "" };

        let severity_label = match diag.severity {
            RenderSeverity::Error => self.formatter.error_color("error"),
            RenderSeverity::Warning => self.formatter.warning_color("warning"),
            RenderSeverity::Note => self.formatter.note_color("note"),
            RenderSeverity::Help => self.formatter.help_color("help"),
        };

        if let Some(ref code) = diag.code {
            writeln!(
                self.writer,
                "{}{}[{}] {}",
                prefix,
                severity_label,
                self.formatter.bold(&code.as_str()),
                diag.message
            )?;
        } else {
            writeln!(self.writer, "{}{} {}", prefix, severity_label, diag.message)?;
        }

        if let Some(ref loc) = diag.location {
            writeln!(
                self.writer,
                "{}  {}",
                prefix,
                self.formatter.file_location(&loc.format_location())
            )?;
        }

        if let Some(ref ctx) = diag.source_context {
            self.render_source_context(ctx, prefix)?;
        }

        for child in &diag.children {
            writeln!(self.writer)?;
            self.render_diagnostic(child, depth + 1)?;
        }

        for suggestion in &diag.suggestions {
            writeln!(
                self.writer,
                "{}  {}",
                prefix,
                self.formatter
                    .help_color(&format!("= help: {}", suggestion))
            )?;
        }

        Ok(())
    }

    fn render_source_context(&mut self, ctx: &SourceContext, prefix: &str) -> io::Result<()> {
        writeln!(
            self.writer,
            "{}   {}",
            prefix,
            self.formatter.file_location(&ctx.file)
        )?;

        for (i, line) in ctx.source_lines.iter().enumerate() {
            let line_num = i + 1;
            writeln!(self.writer, "{}   {} | {}", prefix, line_num, line)?;

            if let Some((start, end)) = ctx.highlight_range {
                if i == 0 {
                    let underline = "   ".to_string()
                        + &" ".repeat(line_num.to_string().len() + 2)
                        + " | "
                        + &" ".repeat(start)
                        + &"^".repeat(end.saturating_sub(start).max(1));
                    writeln!(
                        self.writer,
                        "{}{}",
                        prefix,
                        self.formatter.error_color(&underline)
                    )?;
                }
            }
        }

        Ok(())
    }

    pub fn render_batch(&mut self, diagnostics: &[RenderDiagnostic]) -> io::Result<()> {
        for (i, diag) in diagnostics.iter().enumerate() {
            if i > 0 {
                writeln!(self.writer)?;
            }
            self.render_diagnostic(diag, 0)?;
        }
        Ok(())
    }
}

impl<W: Write> Write for DiagnosticRenderer<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.writer.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

/// Builder for creating render diagnostics from various sources.
pub struct DiagnosticBuilder {
    diagnostic: RenderDiagnostic,
}

impl DiagnosticBuilder {
    pub fn new(severity: RenderSeverity, message: &str) -> Self {
        Self {
            diagnostic: RenderDiagnostic {
                code: None,
                severity,
                message: message.to_string(),
                location: None,
                source_context: None,
                children: Vec::new(),
                suggestions: Vec::new(),
            },
        }
    }

    pub fn error(message: &str) -> Self {
        Self::new(RenderSeverity::Error, message)
    }

    pub fn warning(message: &str) -> Self {
        Self::new(RenderSeverity::Warning, message)
    }

    pub fn note(message: &str) -> Self {
        Self::new(RenderSeverity::Note, message)
    }

    pub fn code(mut self, code: ErrorCode) -> Self {
        self.diagnostic.code = Some(code);
        self
    }

    pub fn at(mut self, location: SourceLocation) -> Self {
        self.diagnostic.location = Some(location);
        self
    }

    pub fn with_context(mut self, context: SourceContext) -> Self {
        self.diagnostic.source_context = Some(context);
        self
    }

    pub fn child(mut self, child: RenderDiagnostic) -> Self {
        self.diagnostic.children.push(child);
        self
    }

    pub fn suggest(mut self, suggestion: &str) -> Self {
        self.diagnostic.suggestions.push(suggestion.to_string());
        self
    }

    pub fn build(self) -> RenderDiagnostic {
        self.diagnostic
    }
}

pub use DiagnosticBuilder as Diag;

/// Platform detection for Windows compatibility.
pub fn is_windows() -> bool {
    cfg!(target_os = "windows")
}

pub fn supports_ansi() -> bool {
    if is_windows() {
        #[cfg(windows)]
        {
            return use_ansi();
        }
        #[cfg(not(windows))]
        {
            return false;
        }
    }
    true
}

#[cfg(windows)]
fn use_ansi() -> bool {
    use std::env;
    env::var("TERM").map(|t| t != "dumb").unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_location_format() {
        let loc = SourceLocation::with_file("test.ry", 10, 5);
        assert_eq!(loc.format_location(), "test.ry:10:5");
    }

    #[test]
    fn test_error_code_in_diagnostic() {
        use crate::diagnostics::codes::ErrorCategory;
        let diag = DiagnosticBuilder::error("Type mismatch")
            .code(ErrorCode::new(ErrorCategory::Type, 3001))
            .at(SourceLocation::new(10, 5))
            .build();

        assert!(diag.code.is_some());
        assert_eq!(diag.code.unwrap().as_str(), "E3001");
    }

    #[test]
    fn test_suggestion_in_diagnostic() {
        let diag = DiagnosticBuilder::error("Cannot assign")
            .suggest("Did you mean to use 'mut'?")
            .build();

        assert_eq!(diag.suggestions.len(), 1);
    }

    #[test]
    fn test_child_diagnostic() {
        let child = DiagnosticBuilder::note("Related note").build();
        let diag = DiagnosticBuilder::error("Main error").child(child).build();

        assert_eq!(diag.children.len(), 1);
    }

    #[test]
    fn test_color_scheme() {
        assert!(ColorScheme::Always.supports_colors());
        assert!(!ColorScheme::Never.supports_colors());
    }
}
