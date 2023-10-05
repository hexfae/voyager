use color_eyre::{eyre::Context, Result};
use surrealdb::{
    engine::remote::ws::{Client, Ws},
    opt::auth::Root,
    Surreal,
};

pub async fn connect_to_database() -> Result<Surreal<Client>> {
    let db = create_surreal_instance().await?;
    sign_in_to_database(&db).await?;
    select_namespace_and_database(&db).await?;
    Ok(db)
}

async fn create_surreal_instance() -> Result<Surreal<Client>> {
    tracing::debug!("connecting to database on localhost:8000");
    let db: Surreal<Client> = Surreal::new::<Ws>("127.0.0.1:8000")
        .await
        .context("connecting to database on localhost:8000")?;
    Ok(db)
}

async fn sign_in_to_database(db: &Surreal<Client>) -> Result<()> {
    tracing::debug!("signing in to database as root:root");
    db.signin(Root {
        username: "root",
        password: "root",
    })
    .await
    .context("signing in to database as root")?;
    Ok(())
}

async fn select_namespace_and_database(db: &Surreal<Client>) -> Result<()> {
    tracing::debug!("using namespace and database voyager");
    db.use_ns("voyager")
        .use_db("voyager")
        .await
        .context("selecting namespace voyager, database voyager")?;
    Ok(())
}
