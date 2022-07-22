use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    forc::cli::run_cli().await?;
    Ok(())
}
