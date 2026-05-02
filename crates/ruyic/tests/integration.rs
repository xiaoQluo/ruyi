#[path = "integration/runner.rs"]
mod runner;

use std::path::Path;

fn main() {
    let cases_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("integration")
        .join("cases");

    println!("Running Ruyi integration tests");
    println!("Cases directory: {:?}", cases_dir);

    let results = runner::run_integration_tests(&cases_dir);

    runner::print_summary(&results);

    let failed = results.values().filter(|r| !r.passed).count();
    if failed > 0 {
        std::process::exit(1);
    }
}