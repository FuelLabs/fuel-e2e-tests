use std::error::Error;

// Exists because we want to be able to target *any* forc revision and because
// cargo doesn't allow you to run a binary from some crate via `cargo run`.
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    forc::cli::run_cli().await?;
    Ok(())
}
