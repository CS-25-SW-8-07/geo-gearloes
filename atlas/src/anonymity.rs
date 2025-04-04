use derive_more::From;
use sqlx::{pool::PoolConnection, Postgres};

use crate::{error::DbError, Bbox};

#[derive(sqlx::FromRow, From)]
#[sqlx(transparent)]
#[allow(dead_code)]
struct Anonymity {
    road_id: i32,
    current_k: f64,
}

#[allow(dead_code)]
pub struct Probability(pub f64);

impl TryFrom<f64> for Probability {
    type Error = &'static str;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        if value.ge(&0.0) && value.le(&1.0) {
            Ok(Probability(value))
        } else {
            Err("Provided value that is not inbetween 0 and 1")
        }
    }
}

#[allow(dead_code)]
pub async fn box_anonymity_query(
    mut conn: PoolConnection<Postgres>,
    bbox: Bbox<f64>,
    limit: Option<u32>,
) -> Result<rusty_roads::Anonymities, DbError> {
    let ((minx, miny), (maxx, maxy)) = bbox;
    let limit = limit.map_or("".into(), |x| format!("limit {x}"));

    let query = format!("WITH box AS (SELECT ST_ENVELOPE( ST_SETSRID(ST_COLLECT(ST_MAKEPOINT($1,$2),ST_MAKEPOINT($3, $4)),4326) ) AS bbox),
roadnetwork AS (SELECT id FROM roads
JOIN box ON ST_INTERSECTS(geom,bbox)),

probunknown AS (
    SELECT road_id, Sum(probability) AS visits
    FROM unknownvisittable
    WHERE road_id IN (SELECT id FROM roadnetwork)
    GROUP BY road_id
),
probknown AS (
    SELECT road_id, COUNT(road_id) AS visits
    FROM knownvisittable
    WHERE road_id IN (SELECT id FROM roadnetwork)
    GROUP BY road_id
),
merged AS (
    SELECT road_id, visits
    FROM probunknown
    UNION ALL
    SELECT road_id, visits
    FROM probknown
)

SELECT road_id, SUM(visits) AS visits
FROM merged
GROUP BY road_id
{limit};");

    let res: Vec<Anonymity> = sqlx::query_as(&query)
        .bind(minx)
        .bind(miny)
        .bind(maxx)
        .bind(maxy)
        .fetch_all(&mut *conn)
        .await
        .map_err(DbError::Sqlx)?;

    Ok(rusty_roads::Anonymities {
        road_id: res.iter().map(|x| x.road_id as u64).collect(),
        current_k: res.iter().map(|x| x.current_k).collect(),
    })
}

#[allow(dead_code)]
pub async fn box_add_unknownvisits(
    mut conn: PoolConnection<Postgres>,
    bbox: Bbox<f64>,
    probability: Probability,
) -> Result<(), DbError> {
    let ((minx, miny), (maxx, maxy)) = bbox;

    let query = String::from("WITH box AS (SELECT ST_ENVELOPE( ST_SETSRID(ST_COLLECT(ST_MAKEPOINT($1, $2),ST_MAKEPOINT($3, $4)),4326) ) AS bbox),
roadnetwork AS (SELECT id FROM roads
JOIN box ON ST_INTERSECTS(geom,bbox))

INSERT INTO unknownvisittable (road_id, time, probability)
SELECT id, NOW(), $5 FROM roadnetwork;");

    sqlx::query(&query)
        .bind(minx)
        .bind(miny)
        .bind(maxx)
        .bind(maxy)
        .bind(probability.0)
        .execute(&mut *conn)
        .await
        .map_err(DbError::Sqlx)?;

    Ok(())
}

#[allow(dead_code)]
pub async fn add_trajectories(
    mut conn: PoolConnection<Postgres>,
    trajs: Vec<Vec<u8>>,
) -> Result<(), DbError> {
    let query = String::from(
        "WITH input_data as (select traj from UNNEST($1) as traj),
trajectory AS (INSERT INTO trajectories (geom)
SELECT ST_GeomFromWKB(traj, 4326) FROM input_data
RETURNING id, geom),
intersected_roads AS (
	SELECT roads.id
	FROM roads, trajectory
	WHERE ST_Intersects(trajectory.geom, roads.geom)
)
INSERT INTO knownvisittable
SELECT ir.id, t.id, NOW()
FROM trajectory as t, intersected_roads as ir;",
    );

    sqlx::query(&query)
        .bind(trajs)
        .execute(&mut *conn)
        .await
        .map_err(DbError::Sqlx)?;

    Ok(())
}

