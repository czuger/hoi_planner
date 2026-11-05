use std::process::ExitCode;

use hoi4_planner::cli::run;

#[tokio::main]
async fn main() -> ExitCode {
    run(std::env::args().skip(1)).await
}
