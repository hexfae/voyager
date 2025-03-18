//! Voyage the Void.

mod incantations;
mod nexus;
mod startup;

#[tokio::main]
async fn main() -> miette::Result<()> {
    let handle = startup::stdout_log();
    startup::create_void_voyager_directories()?;
    let _guard = startup::file_log(&handle);
    startup::serve_voyager().await?;
    Ok(())
}
