use geo_types::{coord, Coord, CoordFloat};
use rusty_roads::*;
use sqlx::{pool::PoolConnection, postgres::{PgPoolOptions, PgRow}, Acquire, PgConnection, Pool, Postgres};

type Bbox<T> = ((T, T), (T, T));

pub async fn bind(conn: &str, max_conn: Option<u32>) -> Result<Pool<Postgres>, sqlx::Error> {
    //TODO: denne funktion kunne evt. også stå for at sætte prepared statements op
    PgPoolOptions::new()
        .max_connections(max_conn.unwrap_or(1))
        .connect_lazy(conn)
}

pub async fn box_query(mut conn: PgConnection, bbox: Bbox<f64>) -> Result<Vec<rusty_roads::Road<f64>>, sqlx::Error> {
    let (minx, miny, maxx, maxy) = (bbox.0 .0, bbox.0 .1, bbox.1 .0, bbox.1 .1);
    let res: Vec<sqlx::postgres::PgRow> = sqlx::query("with box as (select st_envelope( st_setsrid(st_collect(st_makepoint($1,$2),st_makepoint($3,$4)),4326) ) as bbox)
select * from public.gis_osm_roads_free_1
join box on st_intersects(geom,bbox)").bind(minx).bind(miny).bind(maxx).bind(maxy).fetch_all(&mut conn).await?; //TODO: query should be LIMIT'ed, maybe it should be a parameter


    todo!("Not yet implemented")
}

// fn to_road<T: sqlx::Row> (row: T) -> Option<Road<f64>> {
//     let a = row.try_get(index).ok()?;
//     todo!()
// }

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
        let password =
            env::var("PASSWORD").expect("`PASSWORD` environment variable should be set ");
        let address = env::var("ADDRESS").expect("`ADDRESS` environment variable should be set ");
        let database = env::var("DBNAME").expect("`DBNAME` environment variable should be set ");

        let pool = bind(
            &format!(
                "postgres://{}:{}@{}/{}",
                &username, &password, &address, &database
            ),
            Some(1),
        )
        .await;
        assert!(matches!(pool, Ok(x) if x.options().get_max_connections() >= 1))
    }
}
