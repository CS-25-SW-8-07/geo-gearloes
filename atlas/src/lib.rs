use derive_more::From;
use geo_traits::to_geo::ToGeoGeometry;
use geo_types::{Geometry, LineString};
use rusty_roads::*;
use sqlx::{
    pool::PoolConnection,
    postgres::{self, PgPoolOptions},
    FromRow, Pool, Postgres, Row,
};
use wkb::reader::read_wkb;

type Bbox<T> = ((T, T), (T, T));
type _DbRoad = (
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

type DbRoad = (i32, Vec<u8>, i64, i16, String, i16, i16, bool, bool); //TODO: osm id is actually u64, other signed/unsigned funny business

#[derive(sqlx::Type, From)]
#[sqlx(transparent, no_pg_array)]
struct MyRoad(Road);

#[derive(sqlx::FromRow, From)]
#[sqlx(transparent)]
struct MyNameRow(NameRow);

#[derive(sqlx::FromRow, From)]
#[sqlx(transparent)]
struct MyRefManyKey(RefManyKey);

#[derive(sqlx::FromRow, From)]
#[sqlx(transparent)]
struct MyFeatureClassRow(FeatureClassRow);

// hopefully other tables are automatically derivable
impl FromRow<'_, postgres::PgRow> for MyRoad {
    fn from_row(row: &'_ postgres::PgRow) -> Result<Self, sqlx::Error> {
        let ls = wkb_to_linestring(&row.try_get::<Vec<u8>, _>("geom")?).ok_or(
            sqlx::Error::ColumnDecode {
                index: "geom".into(),
                source: Box::new(sqlx::Error::ColumnNotFound("geom".into())),
            },
        )?;
        let direc = |c: &str| match c {
            "B" => Some(Direction::Bidirectional),
            "T" => Some(Direction::Backward),
            "F" => Some(Direction::Forward),
            _ => None,
        };
        let road = Road {
            id: row.try_get::<i32, _>("id")? as rusty_roads::Id,
            geom: ls,
            osm_id: row.try_get::<i64, _>("osm_id")? as u64,
            code: row.try_get::<i16, _>("code")? as u16,
            direction: direc(&row.try_get::<String, _>("oneway")?).expect("msg"),
            maxspeed: row.try_get::<i16, _>("maxspeed")? as u16,
            layer: row.try_get::<i16, _>("layer")?,
            bridge: row.try_get::<bool, _>("bridge")?,
            tunnel: row.try_get::<bool, _>("tunnel")?,
        };
        Ok(MyRoad(road))
    }
}

pub async fn bind(conn: &str, max_conn: Option<u32>) -> Result<Pool<Postgres>, sqlx::Error> {
    //TODO: denne funktion kunne evt. også stå for at sætte prepared statements op
    PgPoolOptions::new()
        .max_connections(max_conn.unwrap_or(1))
        .connect_lazy(conn)
}

#[deprecated = "uses wrong table, use `box_query` instead"]
pub async fn box_query_as(
    mut conn: PoolConnection<Postgres>,
    bbox: Bbox<f64>,
) -> Result<Vec<rusty_roads::Road>, sqlx::Error> {
    let (minx, miny, maxx, maxy) = (bbox.0 .0, bbox.0 .1, bbox.1 .0, bbox.1 .1);
    let res: Vec<_DbRoad> = sqlx::query_as("with box as (select st_envelope( st_setsrid(st_collect(st_makepoint($1,$2),st_makepoint($3,$4)),4326) ) as bbox)
select gid, osm_id, code, fclass, name, ref, oneway, maxspeed, layer, bridge, tunnel, st_asbinary(geom,'NDR') from public.gis_osm_roads_free_1
join box on st_intersects(geom,bbox)").bind(minx).bind(miny).bind(maxx).bind(maxy).fetch_all(&mut *conn).await?; //TODO: query should be LIMIT'ed, maybe it should be a parameter
    let res = res.into_iter().filter_map(to_road).collect::<Vec<_>>(); //TODO: should maybe report on any error in linestring construction
    Ok(res)
}

pub async fn box_query(
    mut conn: PoolConnection<Postgres>,
    bbox: Bbox<f64>,
    limit: Option<u32>,
) -> Result<Vec<rusty_roads::Road>, sqlx::Error> {
    let (minx, miny, maxx, maxy) = (bbox.0 .0, bbox.0 .1, bbox.1 .0, bbox.1 .1);
    let limit = limit.map(|x| format!("limit {x}")).unwrap_or("".into()); //i could not get sql's LIMIT ALL to work, so this is a workaround

    let res: Vec<MyRoad> = sqlx::query_as(&format!("with box as (select st_envelope( st_setsrid(st_collect(st_makepoint($1,$2),st_makepoint($3,$4)),4326) ) as bbox)
select id, st_asbinary(st_geometryn(geom,1),'NDR') as geom, osm_id, code, oneway, maxspeed, layer, bridge, tunnel from roads
join box on st_intersects(geom,bbox)
{limit};")).bind(minx).bind(miny).bind(maxx).bind(maxy).fetch_all(&mut *conn).await?; // multilinestring gets converted to just linestring
    Ok(res.into_iter().map(|x| x.0).collect::<Vec<_>>())
}

fn wkb_to_linestring(bytea: &[u8]) -> Option<LineString<f64>> {
    let a = read_wkb(bytea).ok()?.try_to_geometry()?;
    match a {
        Geometry::LineString(geom) => Some(geom),
        Geometry::MultiLineString(geoms) => Some(geoms.0[0].clone()),
        _ => None,
    }
}

#[deprecated]
fn into_road(road: DbRoad) -> Option<Road> {
    let ls = wkb_to_linestring(&road.1)?;
    let direc = |c: &str| match c {
        "B" => Some(Direction::Bidirectional),
        "T" => Some(Direction::Backward),
        "F" => Some(Direction::Forward),
        _ => None,
    };

    // all data from the danish road dataset are within casting bounds
    let res = Road {
        id: road.0 as rusty_roads::Id,
        geom: ls,
        osm_id: road.2 as u64,
        code: road.3 as u16,
        direction: direc(&road.4)?, //FIXME
        maxspeed: road.5 as u16,
        layer: road.6,
        bridge: road.7,
        tunnel: road.8,
    };
    Some(res)
}

#[deprecated]
fn to_road(row: _DbRoad) -> Option<Road> {
    let ls = wkb_to_linestring(&row.11)?;
    Some(Road {
        id: row.0 as rusty_roads::Id,
        geom: ls,
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
    use super::*;
    use dotenvy::dotenv;
    use std::env;
    use std::sync::LazyLock;
    const CONNCOUNT: u32 = 100;
    // these variables are such that environment variables are only loaded once when running test suite
    static USERNAME: LazyLock<String> = LazyLock::new(|| {
        env::var("DB_USERNAME").expect("`DB_USERNAME` environment variable should be set")
    });
    static PASSWORD: LazyLock<String> = LazyLock::new(|| {
        env::var("DB_PASSWORD").expect("`DB_PASSWORD` environment variable should be set")
    });
    static ADDRESS: LazyLock<String> = LazyLock::new(|| {
        env::var("DB_ADDRESS").expect("`DB_ADDRESS` environment variable should be set")
    });
    static DBNAME: LazyLock<String> = LazyLock::new(|| {
        env::var("DB_NAME").expect("`DB_NAME` environment variable should be set")
    });
    static CONN: LazyLock<String> = LazyLock::new(|| {
        dotenv().expect("failed to read environment variables");
        format!(
            "postgres://{}:{}@{}/{}",
            &*USERNAME, &*PASSWORD, &*ADDRESS, &*DBNAME
        )
    });
    static POOL: LazyLock<Pool<Postgres>> = LazyLock::new(|| {
        async_std::task::block_on(async { bind(&CONN, Some(CONNCOUNT)).await.expect("msg") })
    });

    #[async_std::test]
    async fn it_connects() {
        let pool = bind(&CONN, Some(1)).await;
        assert!(matches!(pool, Ok(x) if x.options().get_max_connections() >= 1))
    }

    #[async_std::test]
    #[ignore = "function uses old table structure"]
    async fn sorry_to_box_in_old() {
        let pool = bind(&CONN, Some(1))
            .await
            .expect("Failed to connect to database, perhaps it is offline");
        let res = box_query_as(
            pool.acquire().await.expect("failed to acquire connection"),
            (
                (9.989492935608991, 57.009_828_137_476_51),
                (9.995526228694693, 57.013_236_271_456_69),
            ),
        )
        .await
        .expect("error in box query");
        assert_eq!(res.len(), 79)
    }

    #[async_std::test]
    async fn sorry_to_box_in() {
        let bbox_cassiopeia = (
            (9.989492935608991, 57.009_828_137_476_51),
            (9.995526228694693, 57.013_236_271_456_69),
        );
        let conn = (*POOL).acquire().await.expect("msg");
        let res = box_query(conn, bbox_cassiopeia, None).await;
        assert!(
            matches!(&res, Ok(x) if x.len()==79),
            "x.len()=={}",
            res.map(|x| x.len()).unwrap_or(0)
        )
    }
}
