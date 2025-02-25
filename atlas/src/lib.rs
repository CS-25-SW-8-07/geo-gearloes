use rusty_roads::*;
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};

pub async fn bind(conn: &str, max_conn: Option<u32>) -> Result<Pool<Postgres>, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(max_conn.unwrap_or(1))
        .connect_lazy(conn)
}

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {

    use dotenvy::dotenv;

    use super::*;
    use std::env;

    // const USERNAME: String = dotenv!("USERNAME");
    // const PASSWORD: String = dotenv!("PASSWORD");
    // const ADDRESS: String = dotenv!("ADDRESS");
    // const DATABASE: String = dotenv!("DATABASE");
    #[async_std::test]
    async fn it_connects() {
        //TODO: ideally, reading environment variables should be done at compile time
        dotenv().expect("Failed to read environment variables");
        let username = env::var("USERNAME").expect("`USERNAME` environment variable should be set");
        let password = env::var("PASSWORD").expect("`PASSWORD` environment variable should be set ");
        let address = env::var("ADDRESS").expect("`ADDRESS` environment variable should be set ");
        let database = env::var("DBNAME").expect("`DBNAME` environment variable should be set ");

        let pool = bind(
            &format!("postgres://{}:{}@{}/{}",&username,&password,&address,&database),
            Some(1),
        )
        .await;
        assert!(matches!(pool, Ok(x) if x.options().get_max_connections() >= 1))
    }

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
