use sqlx::postgres::Postgres;
use sqlx::Pool;

use crate::config::Config;
use crate::Result;

/**
 * create new database connection
 */
pub async fn open_connection(config: &Config) -> Result<Pool<Postgres>> {
    let database_url = config.postgre_url.clone();
    Ok(Pool::<Postgres>::connect(&database_url).await?)
}
