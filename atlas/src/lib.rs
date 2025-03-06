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

#[derive(sqlx::Type, From)]
#[sqlx(transparent, no_pg_array)]
struct MyRoad(Road<f64>);

#[derive(sqlx::FromRow, From)]
#[sqlx(transparent)]
#[allow(dead_code)]
struct MyNameRow(NameRow);

#[derive(sqlx::FromRow, From)]
#[sqlx(transparent)]
#[allow(dead_code)]
struct MyRefManyKey(RefManyKey);

#[derive(sqlx::FromRow, From)]
#[sqlx(transparent)]
#[allow(dead_code)]
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
        let road = Road::<f64> {
            id: row.try_get::<i32, _>("id")? as usize,
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

pub async fn box_query(
    mut conn: PoolConnection<Postgres>,
    bbox: Bbox<f64>,
    limit: Option<u32>,
) -> Result<Vec<rusty_roads::Road<f64>>, sqlx::Error> {
    let (minx, miny, maxx, maxy) = (bbox.0 .0, bbox.0 .1, bbox.1 .0, bbox.1 .1);
    let limit = limit.map(|x| format!("limit {x}")).unwrap_or("".into()); //i could not get sql's LIMIT ALL to work, so this is a workaround

    let sql = format!("with box as (select st_envelope( st_setsrid(st_collect(st_makepoint($1,$2),st_makepoint($3,$4)),4326) ) as bbox)
    select id, st_asbinary(st_geometryn(geom,1),'NDR') as geom, osm_id, code, oneway, maxspeed, layer, bridge, tunnel from roads
    join box on st_intersects(geom,bbox)
    {limit};");

    let res: Vec<MyRoad> = sqlx::query_as(&sql)
        .bind(minx)
        .bind(miny)
        .bind(maxx)
        .bind(maxy)
        .fetch_all(&mut *conn)
        .await?; // multilinestring gets converted to just linestring
    Ok(res.into_iter().map(|x| x.0).collect::<Vec<_>>())
}

pub async fn box_query_without(
    mut conn: PoolConnection<Postgres>,
    bbox: Bbox<f64>,
    without: &[usize],
    limit: Option<u32>,
) -> Result<Vec<rusty_roads::Road<f64>>, sqlx::Error> {
    let (minx, miny, maxx, maxy) = (bbox.0 .0, bbox.0 .1, bbox.1 .0, bbox.1 .1);
    let limit = limit.map(|x| format!("limit {x}")).unwrap_or("".into());

    let where_clause = match without.len() {
        0 => "".into(),
        _ => {
            let not_in = without
                .iter()
                // .fold("", |acc,x| format!("{acc},{x},"));
                .map(|id| format!("{},", id))
                .collect::<String>();
            let not_in = &not_in[0..not_in.len() - 1];
            format!("where id not in ({not_in})")
        }
    };

    let sql = format!("with box as (select st_envelope( st_setsrid(st_collect(st_makepoint($1,$2),st_makepoint($3,$4)),4326) ) as bbox)
    select id, st_asbinary(st_geometryn(geom,1),'NDR') as geom, osm_id, code, oneway, maxspeed, layer, bridge, tunnel from roads
    join box on st_intersects(geom,bbox)  {where_clause} 
    {limit};");
    let res: Vec<MyRoad> = sqlx::query_as(&sql)
        .bind(minx)
        .bind(miny)
        .bind(maxx)
        .bind(maxy)
        .fetch_all(&mut *conn)
        .await?;

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
        async_std::task::block_on(async { bind(&*CONN, Some(CONNCOUNT)).await.expect("msg") })
    });

    static BBOX_CASSIOPEIA: LazyLock<Bbox<f64>> = LazyLock::new(|| {
        (
            (9.989492935608991, 57.009828137476511),
            (9.995526228694693, 57.013236271456691),
        )
    });

    #[async_std::test]
    async fn it_connects() {
        let pool = bind(&*CONN, Some(1)).await;
        assert!(matches!(pool, Ok(x) if x.options().get_max_connections() >= 1))
    }

    mod box_query {
        use super::*;

        #[async_std::test]
        async fn sorry_to_box_in() {
            let conn = (*POOL)
                .acquire()
                .await
                .expect("failed to establish database connection, perhaps it is closed");
            let res = box_query(conn, *BBOX_CASSIOPEIA, None).await;
            assert!(
                matches!(&res, Ok(x) if x.len()==79),
                "x.len()=={}",
                res.map(|x| x.len()).unwrap_or(0)
            )
        }
    }

    mod box_query_without {
        use super::*;

        #[async_std::test]
        async fn box_without() {
            let without = [592125, 592124, 661737];
            let conn = (*POOL)
                .acquire()
                .await
                .expect("failed to establish database connection, perhaps it is closed");
            let res = box_query_without(conn, *BBOX_CASSIOPEIA, &without, None)
                .await
                .expect("failed to execute query");
            assert_eq!(79 - without.len(), res.len())
        }
    }
}
