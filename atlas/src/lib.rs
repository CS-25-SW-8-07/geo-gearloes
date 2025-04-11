pub mod anonymity;
pub mod model;
mod error;

use derive_more::From;
use error::DbError;
use geo_traits::to_geo::ToGeoGeometry;
use geo_types::{Geometry, LineString};
use rusty_roads::*;
use sqlx::{
    pool::PoolConnection,
    postgres::{self, PgPoolOptions},
    FromRow, Pool, Postgres, Row,
};
use std::fmt::Write;
use wkb::reader::read_wkb;

type Bbox<T> = ((T, T), (T, T));

#[derive(sqlx::Type, From)]
#[sqlx(transparent, no_pg_array)]
struct MyRoad(Road);

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
        let id = row.try_get::<i32, _>("id")? as rusty_roads::Id;
        let ls = wkb_to_linestring(&row.try_get::<Vec<u8>, _>("geom")?).ok_or(
            sqlx::Error::ColumnDecode {
                index: "geom".into(),
                source: Box::new(DbError::Linestring(id)),
            },
        )?;
        let direc = |c: &str| match c {
            "B" => Ok(Direction::Bidirectional),
            "T" => Ok(Direction::Backward),
            "F" => Ok(Direction::Forward),
            e => Err(sqlx::Error::ColumnDecode {
                index: "oneway".into(),
                source: Box::new(DbError::DirectionDecode(e.into())),
            }),
        };

        let road = Road {
            id,
            geom: ls,
            osm_id: row.try_get::<i64, _>("osm_id")? as u64,
            code: row.try_get::<i16, _>("code")? as u16,
            direction: direc(&row.try_get::<String, _>("oneway")?)?,
            maxspeed: row.try_get::<i16, _>("maxspeed")? as u16,
            layer: row.try_get::<i16, _>("layer")?,
            bridge: row.try_get::<bool, _>("bridge")?,
            tunnel: row.try_get::<bool, _>("tunnel")?,
        };
        Ok(MyRoad(road))
    }
}

/// Lazily creates a connection [`Pool`] with a given maximum connections (default 1).
///
/// # Errors
///
/// This function will return an error if the connection issues (e.g. malformed connection string, or if the database is down).
pub async fn create_pool(conn: &str, max_conn: Option<u32>) -> Result<Pool<Postgres>, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(max_conn.unwrap_or(1))
        .connect_lazy(conn)
}

/// Retrives [`Road`]s that intersect with a given bounding box, up to a limit, if given.
///
/// # Errors
///
/// This function will return an error if there are connection issues with the database.
pub async fn box_query(
    conn: PoolConnection<Postgres>,
    bbox: Bbox<f64>,
    limit: Option<u32>,
) -> Result<Vec<rusty_roads::Road>, DbError> {
    box_query_exclude_by_id(conn, bbox, &[], limit).await
}

/// Like [`box_query`], but allows excluding certain roads by id.
///
/// # Errors
///
/// This function will return an error if there are connection issues with the database.
pub async fn box_query_exclude_by_id(
    mut conn: PoolConnection<Postgres>,
    bbox: Bbox<f64>,
    without: &[usize],
    limit: Option<u32>,
) -> Result<Vec<rusty_roads::Road>, DbError> {
    let ((minx, miny), (maxx, maxy)) = bbox;
    let limit = limit.map_or("".into(), |x| format!("limit {x}"));

    let where_clause = match without.len() {
        0 => "".into(),
        _ => {
            let not_in = without.iter().fold(String::new(), |mut acc, id| {
                let _ = write!(acc, "{id},");
                acc
            });
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
        .await
        .map_err(DbError::Sqlx)?;

    Ok(res.into_iter().map(|x| x.0).collect::<Vec<_>>())
}

fn wkb_to_linestring(bytea: &[u8]) -> Option<LineString<f64>> {
    let a = read_wkb(bytea).ok()?.try_to_geometry()?;
    match a {
        Geometry::LineString(geom) => Some(geom), // empty linestringes are still allowed
        Geometry::MultiLineString(geoms) => Some(geoms.0.get(0)?.clone()), // to avoid this, in sql, write st_geomtryn(geom,1)
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
        async_std::task::block_on(async {
            create_pool(&*CONN, Some(CONNCOUNT)).await.expect("msg")
        })
    });

    static BBOX_CASSIOPEIA: LazyLock<Bbox<f64>> = LazyLock::new(|| {
        (
            (9.989492935608991, 57.009828137476511),
            (9.995526228694693, 57.013236271456691),
        )
    });
    const BBOX_CASSIOPEIA_COUNT: usize = 79;
    #[async_std::test]
    async fn it_connects() {
        let pool = create_pool(&*CONN, Some(1)).await;
        assert!(matches!(pool, Ok(x) if x.options().get_max_connections() >= 1))
    }

    mod box_query {
        use std::f64::NAN;

        use super::*;

        #[async_std::test]
        async fn sorry_to_box_in() {
            let conn = (*POOL)
                .acquire()
                .await
                .expect("failed to establish database connection, perhaps it is closed");
            let res = box_query(conn, *BBOX_CASSIOPEIA, None).await;
            assert!(
                matches!(&res, Ok(x) if x.len()==BBOX_CASSIOPEIA_COUNT),
                "x.len()=={}",
                res.map(|x| x.len()).unwrap_or(0)
            )
        }
        #[async_std::test]
        async fn sorry_to_box_in_illegal_box() {
            let bbox = ((NAN, NAN), (NAN, NAN));
            const NAN_BOX: usize = 0;
            let conn = (*POOL)
                .acquire()
                .await
                .expect("failed to establish database connection, perhaps it is closed");
            let res = box_query(conn, bbox, None)
                .await
                .expect("failed to execute query");
            assert_eq!(NAN_BOX, res.len())
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
            let res = box_query_exclude_by_id(conn, *BBOX_CASSIOPEIA, &without, None)
                .await
                .expect("failed to execute query");
            assert_eq!(BBOX_CASSIOPEIA_COUNT - without.len(), res.len())
        }

        #[async_std::test]
        async fn box_without_but_empty() {
            let conn = (*POOL)
                .acquire()
                .await
                .expect("failed to establish database connection, perhaps it is closed");
            let res = box_query_exclude_by_id(conn, *BBOX_CASSIOPEIA, &[], None)
                .await
                .expect("failed to execute query");
            assert_eq!(BBOX_CASSIOPEIA_COUNT, res.len())
        }
    }
}
