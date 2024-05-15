mod postgres;
mod sqlite;

use postgres::PostgresRepository;
use sqlite::SqliteRepository;

pub trait Repository {}
