use color_eyre::Result;
use surrealdb::{
    engine::remote::ws::{Client, Ws},
    opt::auth::Root,
    Surreal,
};

// TODO: fix documentation
/// Creates/uses an embedded SpeeDb database in `$PWD/voyager.db`
/// in release mode or runs database in memory in debug mode.
pub async fn connect_to_database() -> Result<Surreal<Client>> {
    // TODO: change to 8000 or make it an environment variable or something
    let db = Surreal::new::<Ws>("127.0.0.1:8002").await?;
    db.use_ns("voyager").use_db("voyager").await?;
    db.signin(Root {
        username: "root",
        password: "root",
    })
    .await?;
    Ok(db)
}
