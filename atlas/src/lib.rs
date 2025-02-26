use geo_traits::{
    to_geo::{self, ToGeoGeometry},
    GeometryTrait,
};
use geo_types::{coord, Coord, CoordFloat, Geometry, LineString};
use rusty_roads::*;
use sqlx::{
    pool::PoolConnection,
    postgres::{PgPoolOptions, PgRow},
    Acquire, PgConnection, Pool, Postgres,
};
use wkb::reader::read_wkb;

type Bbox<T> = ((T, T), (T, T));

pub async fn bind(conn: &str, max_conn: Option<u32>) -> Result<Pool<Postgres>, sqlx::Error> {
    //TODO: denne funktion kunne evt. også stå for at sætte prepared statements op
    PgPoolOptions::new()
        .max_connections(max_conn.unwrap_or(1))
        .connect_lazy(conn)
}

#[deprecated="box_query_as is better"]
pub async fn box_query(
    mut conn: PgConnection,
    bbox: Bbox<f64>,
) -> Result<Vec<rusty_roads::Road<f64>>, sqlx::Error> {
    let (minx, miny, maxx, maxy) = (bbox.0 .0, bbox.0 .1, bbox.1 .0, bbox.1 .1);
    let res: Vec<sqlx::postgres::PgRow> = sqlx::query("with box as (select st_envelope( st_setsrid(st_collect(st_makepoint($1,$2),st_makepoint($3,$4)),4326) ) as bbox)
select * from public.gis_osm_roads_free_1
join box on st_intersects(geom,bbox)").bind(minx).bind(miny).bind(maxx).bind(maxy).fetch_all(&mut conn).await?; //TODO: query should be LIMIT'ed, maybe it should be a parameter

    todo!("Not yet implemented")
}

type DbRoad = (
    i32,
    String,
    i16,
    String,
    Option<String>,
    Option<String>,
    String,
    i16,
    f64,
    String,
    String,
    Vec<u8>,
);
pub async fn box_query_as(
    mut conn: PoolConnection<Postgres>,
    bbox: Bbox<f64>,
) -> Result<Vec<rusty_roads::Road<f64>>, sqlx::Error> {
    let (minx, miny, maxx, maxy) = (bbox.0 .0, bbox.0 .1, bbox.1 .0, bbox.1 .1);
    let res: Vec<DbRoad> = sqlx::query_as("with box as (select st_envelope( st_setsrid(st_collect(st_makepoint($1,$2),st_makepoint($3,$4)),4326) ) as bbox)
select gid, osm_id, code, fclass, name, ref, oneway, maxspeed, layer, bridge, tunnel, st_asbinary(geom,'NDR') from public.gis_osm_roads_free_1
join box on st_intersects(geom,bbox)").bind(minx).bind(miny).bind(maxx).bind(maxy).fetch_all(&mut *conn).await?; //TODO: query should be LIMIT'ed, maybe it should be a parameter
    let res = res
        .into_iter()
        .filter_map(|s| to_road(s))
        .collect::<Vec<_>>(); //TODO: should maybe report on any error in linestring construction
    Ok(res)
}

fn wkb_to_linestring(bytea: &[u8]) -> Option<LineString<f64>> {
    let a = read_wkb(bytea).ok()?.try_to_geometry()?;
    match a {
        Geometry::LineString(geom) => Some(geom),
        _ => None,
    }
}

fn to_road(row: DbRoad) -> Option<Road<f64>> {
    //let linestring = LineString::<f64>::try_from( read_wkb(&row.11).ok()?);
    Some(Road {
        id: row.0 as usize,
        geom: wkb_to_linestring(&row.11)?,
        osm_id: row.1.parse::<u64>().ok()?,
        code: row.2 as u16,                  //FIXME
        direction: Direction::Bidirectional, //FIXME
        maxspeed: row.7 as u16,              //FIXME
        layer: row.8 as i16,                 //TODO: probably an error in schema
        bridge: true,                        //FIXME
        tunnel: true,                        //FIXME
    })
}

#[cfg(test)]
mod tests {

    use dotenvy::dotenv;

    use super::*;
    use std::env;
    use std::sync::LazyLock;
    static USERNAME: LazyLock<String> = LazyLock::new(|| {
        env::var("USERNAME").expect("`USERNAME` environment variable should be set")
    });
    static PASSWORD: LazyLock<String> = LazyLock::new(|| {
        env::var("PASSWORD").expect("`PASSWORD` environment variable should be set")
    });
    static ADDRESS: LazyLock<String> = LazyLock::new(|| {
        env::var("ADDRESS").expect("`ADDRESS` environment variable should be set")
    });
    static DBNAME: LazyLock<String> =
        LazyLock::new(|| env::var("DBNAME").expect("`DBNAME` environment variable should be set"));
    static CONN: LazyLock<String> = LazyLock::new(|| {
        dotenv().expect("failed to read environment variables");
        format!(
            "postgres://{}:{}@{}/{}",
            &*USERNAME, &*PASSWORD, &*ADDRESS, &*DBNAME
        )
    });
    #[async_std::test]
    async fn it_connects() {
        let pool = bind(&*CONN, Some(1)).await;
        assert!(matches!(pool, Ok(x) if x.options().get_max_connections() >= 1))
    }

    #[async_std::test]
    async fn sorry_to_box_in() {
        let pool = bind(&*CONN, Some(1))
            .await
            .expect("Failed to connect to database, perhaps it is offline");
        let res = box_query_as(
            pool.acquire().await.expect("failed to acquire connection"),
            (
                (9.989492935608991, 57.009828137476511),
                (9.995526228694693, 57.013236271456691),
            ),
        )
        .await
        .expect("error in box query");
        assert_eq!(res.len(),79)
    }
}
