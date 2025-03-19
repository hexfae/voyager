//! Voyage the Void.

mod incantations;
mod nexus;
mod startup;

#[tokio::main]
async fn main() -> miette::Result<()> {
    // begin logging to stdout
    let handle = startup::stdout_log();
    // create voyager/backups, voyager/logs
    startup::create_void_voyager_directories()?;
    // begin logging to voyager/logs/voyager.log.yyyy-mm-dd
    let _guard = startup::file_log(&handle);
    // load the nexus
    let nexus = nexus::Nexus::try_load()?;
    // save to update the config file
    nexus.try_save()?;
    // TODO: webui
    // watch the config file for edits to hot reload
    let mut debouncer = startup::create_debouncer(nexus.manifest.clone())?;
    startup::WatchVoyagerConfig::watch_voyager_config(&mut debouncer)?;
    // backup levels daily
    tokio::spawn(startup::backup_levels_daily(nexus.atlas.clone()));
    // serve voyager
    startup::serve_voyager(nexus).await
}
