use std::collections::HashMap;
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

#[derive(Debug, Clone)]
pub struct TestCase {
    pub id: String,
    pub source_path: PathBuf,
    pub expected_path: Option<PathBuf>,
    pub error_path: Option<PathBuf>,
    pub category: String,
}

#[derive(Debug, Clone)]
pub struct TestResult {
    pub test_id: String,
    pub passed: bool,
    pub message: String,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
}

pub struct DiscoveryResult {
    pub tests: Vec<TestCase>,
}

pub fn discover_tests(cases_dir: &Path) -> io::Result<DiscoveryResult> {
    let mut tests = Vec::new();

    if !cases_dir.exists() {
        return Ok(DiscoveryResult { tests });
    }

    for entry in fs::read_dir(cases_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            let category = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            for test_entry in fs::read_dir(&path)? {
                let test_entry = test_entry?;
                let test_path = test_entry.path();

                if test_path.extension() == Some(OsStr::new("ry")) {
                    let test_name = test_path.file_stem()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unnamed")
                        .to_string();

                    let test_id = format!("{}/{}", category, test_name);

                    let expected_path = test_path.with_extension("expected");
                    let error_path = test_path.with_extension("error");

                    let test_case = TestCase {
                        id: test_id,
                        source_path: test_path,
                        expected_path: if expected_path.exists() {
                            Some(expected_path)
                        } else {
                            None
                        },
                        error_path: if error_path.exists() {
                            Some(error_path)
                        } else {
                            None
                        },
                        category: category.clone(),
                    };

                    tests.push(test_case);
                }
            }
        }
    }

    tests.sort_by(|a, b| a.id.cmp(&b.id));

    Ok(DiscoveryResult { tests })
}

pub fn run_positive_test(test: &TestCase, ruyic_path: &Path) -> TestResult {
    let test_id = &test.id;
    let source_path = &test.source_path;
    let expected_path = test.expected_path.as_ref().expect("positive test must have .expected file");

    let temp_dir = env::temp_dir();
    let binary_path = temp_dir.join(format!("ruyi_test_{}", test_id.replace("/", "_")));

    let compile_result = compile_file(source_path, &binary_path, ruyic_path);

    if !compile_result.status.success() {
        let stderr = String::from_utf8_lossy(&compile_result.stderr).to_string();
        return TestResult {
            test_id: test_id.clone(),
            passed: false,
            message: format!("Compilation failed:\n{}", stderr),
            stdout: None,
            stderr: Some(stderr),
        };
    }

    let run_result = run_binary(&binary_path);

    let _ = fs::remove_file(&binary_path);

    if !run_result.status.success() {
        let stderr = String::from_utf8_lossy(&run_result.stderr).to_string();
        return TestResult {
            test_id: test_id.clone(),
            passed: false,
            message: format!("Execution failed (exit code: {:?}):\n{}",
                run_result.status.code(), stderr),
            stdout: Some(String::from_utf8_lossy(&run_result.stdout).to_string()),
            stderr: Some(stderr),
        };
    }

    let expected = match fs::read_to_string(expected_path) {
        Ok(content) => content,
        Err(e) => return TestResult {
            test_id: test_id.clone(),
            passed: false,
            message: format!("Failed to read .expected file: {}", e),
            stdout: None,
            stderr: None,
        },
    };

    let actual = String::from_utf8_lossy(&run_result.stdout).to_string();

    let expected_normalized = expected.replace("\r\n", "\n").trim().to_string();
    let actual_normalized = actual.replace("\r\n", "\n").trim().to_string();

    if expected_normalized == actual_normalized {
        TestResult {
            test_id: test_id.clone(),
            passed: true,
            message: "Test passed".to_string(),
            stdout: Some(actual),
            stderr: None,
        }
    } else {
        TestResult {
            test_id: test_id.clone(),
            passed: false,
            message: format!(
                "Output mismatch:\nExpected:\n{}\n\nActual:\n{}",
                expected_normalized, actual_normalized
            ),
            stdout: Some(actual),
            stderr: None,
        }
    }
}

pub fn run_negative_test(test: &TestCase, ruyic_path: &Path) -> TestResult {
    let test_id = &test.id;
    let source_path = &test.source_path;
    let error_path = test.error_path.as_ref().expect("negative test must have .error file");

    let compile_result = compile_file(source_path, Path::new("/dev/null"), ruyic_path);

    if compile_result.status.success() {
        return TestResult {
            test_id: test_id.clone(),
            passed: false,
            message: "Compilation succeeded but was expected to fail".to_string(),
            stdout: None,
            stderr: Some(String::from_utf8_lossy(&compile_result.stderr).to_string()),
        };
    }

    let expected_error = match fs::read_to_string(error_path) {
        Ok(content) => content.trim().to_string(),
        Err(e) => return TestResult {
            test_id: test_id.clone(),
            passed: false,
            message: format!("Failed to read .error file: {}", e),
            stdout: None,
            stderr: None,
        },
    };

    let actual_stderr = String::from_utf8_lossy(&compile_result.stderr).to_string();

    if actual_stderr.contains(&expected_error) {
        TestResult {
            test_id: test_id.clone(),
            passed: true,
            message: "Negative test passed (compilation failed as expected)".to_string(),
            stdout: None,
            stderr: Some(actual_stderr),
        }
    } else {
        TestResult {
            test_id: test_id.clone(),
            passed: false,
            message: format!(
                "Error message mismatch:\nExpected pattern:\n{}\n\nActual stderr:\n{}",
                expected_error, actual_stderr
            ),
            stdout: None,
            stderr: Some(actual_stderr),
        }
    }
}

fn compile_file(source: &Path, output: &Path, ruyic_path: &Path) -> Output {
    Command::new(ruyic_path)
        .arg(source)
        .arg("-o")
        .arg(output)
        .output()
        .expect("Failed to execute ruyic")
}

fn run_binary(binary: &Path) -> Output {
    Command::new(binary)
        .output()
        .expect("Failed to execute binary")
}

pub fn get_ruyic_path() -> PathBuf {
    env::var("KLANG_BIN")
        .map(PathBuf::from)
        .ok()
        .filter(|p| p.exists())
        .unwrap_or_else(|| {
            env::var("CARGO_BIN_EXE_ruyic")
                .map(PathBuf::from)
                .ok()
                .filter(|p| p.exists())
                .unwrap_or_else(|| {
                    let debug_path = PathBuf::from("target/debug/ruyic");
                    if debug_path.exists() {
                        debug_path
                    } else {
                        PathBuf::from("target/release/ruyic")
                    }
                })
        })
}

pub fn run_integration_tests(cases_dir: &Path) -> HashMap<String, TestResult> {
    let mut results = HashMap::new();
    let ruyic_path = get_ruyic_path();
    if !ruyic_path.exists() {
        eprintln!("Warning: ruyic not found at {:?}", ruyic_path);
        eprintln!("Set KLANG_BIN environment variable or build with `cargo build`");
        return results;
    }

    let discovery = match discover_tests(cases_dir) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Failed to discover tests: {}", e);
            return results;
        }
    };

    println!("Discovered {} test cases", discovery.tests.len());

    for test in &discovery.tests {
        let result = if test.error_path.is_some() {
            run_negative_test(test, &ruyic_path)
        } else if test.expected_path.is_some() {
            run_positive_test(test, &ruyic_path)
        } else {
            TestResult {
                test_id: test.id.clone(),
                passed: false,
                message: "Test case has neither .expected nor .error file".to_string(),
                stdout: None,
                stderr: None,
            }
        };

        let status = if result.passed { "PASS" } else { "FAIL" };
        println!("[{}] {}", status, test.id);

        if !result.passed {
            println!("  Message: {}", result.message.split('\n').next().unwrap_or(&result.message));
        }

        results.insert(test.id.clone(), result);
    }

    results
}

pub fn print_summary(results: &HashMap<String, TestResult>) {
    let total = results.len();
    let passed = results.values().filter(|r| r.passed).count();
    let failed = total - passed;

    println!("\n========================================");
    println!("Test Summary");
    println!("========================================");
    println!("Total:  {}", total);
    println!("Passed: {}", passed);
    println!("Failed: {}", failed);

    if failed > 0 {
        println!("\nFailed tests:");
        for (id, result) in results.iter().filter(|(_, r)| !r.passed) {
            println!("  - {}", id);
            println!("    Reason: {}", result.message.split('\n').next().unwrap_or(&result.message));
        }
    }
}

pub fn format_results(results: &HashMap<String, TestResult>) -> String {
    let mut output = String::new();

    for (id, result) in results.iter() {
        output.push_str(&format!("{}: {}\n", id, if result.passed { "ok" } else { "FAILED" }));
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discover_tests() {
        let temp_dir = env::temp_dir();
        let test_dir = temp_dir.join("ruyi_test_discovery");

        fs::create_dir_all(test_dir.join("basic")).ok();

        fs::write(test_dir.join("basic/hello.ry"), "print(\"hello\")").ok();
        fs::write(test_dir.join("basic/hello.expected"), "hello").ok();
        fs::write(test_dir.join("basic/bad.ry"), "let x =").ok();
        fs::write(test_dir.join("basic/bad.error"), "expected").ok();

        let discovery = discover_tests(&test_dir).unwrap();

        assert_eq!(discovery.tests.len(), 2);

        fs::remove_dir_all(test_dir).ok();
    }

    #[test]
fn test_get_ruyic_path() {
    let path = get_ruyic_path();
        assert!(path.file_name().is_some());
    }
}