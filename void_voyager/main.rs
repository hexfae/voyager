//! Voyage the Void.

mod incantations;
mod nexus;
mod startup;

#[tokio::main]
async fn main() -> miette::Result<()> {
    let handle = startup::stdout_log();
    startup::create_void_voyager_directories()?;
    let _guard = startup::file_log(&handle);
    let nexus = nexus::Nexus::try_load()?;
    nexus.try_save()?;
    let mut debouncer = startup::watch_config(nexus.manifest.clone())?;
    startup::WatchVoyagerConfig::watch_voyager_config(&mut debouncer)?;
    startup::serve_voyager(nexus).await?;
    Ok(())
}
