use ruyic::diagnostics::{codes::*, render::*, DiagnosticBuilder, SourceContext, SourceLocation};

fn type_mismatch_error() -> RenderDiagnostic {
    DiagnosticBuilder::error("mismatched types")
        .code(TypeErrorCode::TypeMismatch.to_error_code())
        .at(SourceLocation::with_file("test.ry", 5, 10))
        .with_context(SourceContext::single_line(
            "test.ry",
            "    let x: int = \"hello\";",
            (15, 23),
        ))
        .child(DiagnosticBuilder::note("expected `int`, found `string`").build())
        .suggest("cast to int with `as int`")
        .build()
}

fn undefined_variable_error() -> RenderDiagnostic {
    DiagnosticBuilder::error("cannot find value")
        .code(ResolutionErrorCode::UnknownVariable.to_error_code())
        .at(SourceLocation::with_file("test.ry", 10, 5))
        .with_context(SourceContext::single_line(
            "test.ry",
            "    println(count);",
            (13, 18),
        ))
        .suggest("did you mean `cound`?")
        .build()
}

fn missing_semicolon_error() -> RenderDiagnostic {
    DiagnosticBuilder::error("expected `;`")
        .code(SyntaxErrorCode::MissingSemicolon.to_error_code())
        .at(SourceLocation::with_file("test.ry", 3, 14))
        .with_context(SourceContext::single_line(
            "test.ry",
            "    let x = 42",
            (14, 14),
        ))
        .build()
}

fn unused_variable_warning() -> RenderDiagnostic {
    DiagnosticBuilder::warning("unused variable `unused`")
        .code(WarningCode::UnusedVariable.to_error_code())
        .at(SourceLocation::with_file("test.ry", 1, 5))
        .with_context(SourceContext::single_line(
            "test.ry",
            "let unused = 42;",
            (4, 10),
        ))
        .build()
}

fn unreachable_code_warning() -> RenderDiagnostic {
    DiagnosticBuilder::warning("unreachable code")
        .code(WarningCode::UnreachableCode.to_error_code())
        .at(SourceLocation::with_file("test.ry", 15, 5))
        .build()
}

fn assert_renders(diag: RenderDiagnostic, expected: &str) {
    let mut output = Vec::new();
    {
        let mut renderer = DiagnosticRenderer::new(&mut output, ColorScheme::Never);
        renderer.render(&diag).unwrap();
    }
    let result = String::from_utf8(output).unwrap();
    assert_eq!(result.trim(), expected.trim());
}

#[test]
fn test_type_mismatch_renders() {
    assert_renders(
        type_mismatch_error(),
        r#"error[E3001] mismatched types
  test.ry:5:10
   test.ry
   1 |     let x: int = "hello";
                           ^^^^^^
      expected `int`, found `string`

  = help: cast to int with `as int`"#,
    );
}

#[test]
fn test_undefined_variable_renders() {
    assert_renders(
        undefined_variable_error(),
        r#"error[E4001] cannot find value
  test.ry:10:5
   test.ry
   1 |     println(count);
                   ^^^^^
  = help: did you mean `cound`?"#,
    );
}

#[test]
fn test_missing_semicolon_renders() {
    assert_renders(
        missing_semicolon_error(),
        r#"error[E2006] expected `;`
  test.ry:3:14
   test.ry
   1 |     let x = 42
                  ^
"#,
    );
}

#[test]
fn test_unused_variable_warning_renders() {
    assert_renders(
        unused_variable_warning(),
        r#"warning[W1001] unused variable `unused`
  test.ry:1:5
   test.ry
   1 | let unused = 42;
             ^^^^^^
"#,
    );
}

#[test]
fn test_unreachable_code_warning_renders() {
    assert_renders(
        unreachable_code_warning(),
        r#"warning[W1002] unreachable code
  test.ry:15:5
"#,
    );
}

#[test]
fn test_error_chain_renders() {
    let primary = DiagnosticBuilder::error("type mismatch")
        .code(TypeErrorCode::TypeMismatch.to_error_code())
        .at(SourceLocation::with_file("main.ry", 20, 8))
        .with_context(SourceContext::single_line(
            "main.ry",
            "    let result = add(\"hi\", 42);",
            (13, 25),
        ))
        .child(DiagnosticBuilder::note("arguments to this function are...").build())
        .child(DiagnosticBuilder::note("...expected `string`, found `int`").build())
        .build();

    assert_renders(
        primary,
        r#"error[E3001] type mismatch
  main.ry:20:8
   main.ry
   1 |     let result = add("hi", 42);
                            ^^^^^^^^^^
  note
  main.ry:20:8
   ...arguments to this function are...
  note
  main.ry:20:8
   ...expected `string`, found `int`"#,
    );
}

#[test]
fn test_colorized_output() {
    let diag = DiagnosticBuilder::error("test error")
        .code(ErrorCode::new(ErrorCategory::Syntax, 2001))
        .at(SourceLocation::with_file("test.ry", 1, 1))
        .build();

    let mut colored_output = Vec::new();
    {
        let mut renderer = DiagnosticRenderer::new(&mut colored_output, ColorScheme::Always);
        renderer.render(&diag).unwrap();
    }
    let result = String::from_utf8(colored_output).unwrap();
    assert!(result.contains("\x1b[")); // Contains ANSI codes
}

#[test]
fn test_no_color_on_never() {
    let diag = DiagnosticBuilder::error("test error")
        .at(SourceLocation::with_file("test.ry", 1, 1))
        .build();

    let mut output = Vec::new();
    {
        let mut renderer = DiagnosticRenderer::new(&mut output, ColorScheme::Never);
        renderer.render(&diag).unwrap();
    }
    let result = String::from_utf8(output).unwrap();
    assert!(!result.contains("\x1b["));
}

#[test]
fn test_error_code_display() {
    let code = TypeErrorCode::TypeMismatch.to_error_code();
    assert_eq!(code.as_str(), "E3001");
    assert_eq!(format!("{}", code), "E3001");
}

#[test]
fn test_warning_code_display() {
    let code = WarningCode::UnusedVariable.to_error_code();
    assert_eq!(code.as_str(), "W1001");
}

#[test]
fn test_severity_colors() {
    let formatter = ConsoleFormatter::new(ColorScheme::Never);

    assert_eq!(formatter.error_color("error"), "error");
    assert_eq!(formatter.warning_color("warning"), "warning");
    assert_eq!(formatter.note_color("note"), "note");
}

#[test]
fn test_multi_line_source_context() {
    let ctx = SourceContext::multi_line(
        "test.ry",
        &["fn foo() {", "    let x = 1;", "    return x;", "}"],
        2,
        (4, 13),
    );

    assert_eq!(ctx.file, "test.ry");
    assert_eq!(ctx.source_lines.len(), 4);
}

#[test]
fn test_location_span() {
    let loc = SourceLocation::span(5, 10, 5, 20);
    assert_eq!(loc.line, 5);
    assert_eq!(loc.column, 10);
    assert_eq!(loc.end_line, Some(5));
    assert_eq!(loc.end_column, Some(20));
}

#[test]
fn test_location_format_with_file() {
    let loc = SourceLocation::with_file("src/main.ry", 10, 5);
    assert_eq!(loc.format_location(), "src/main.ry:10:5");
}

#[test]
fn test_location_format_without_file() {
    let loc = SourceLocation::new(10, 5);
    assert_eq!(loc.format_location(), "10:5");
}
