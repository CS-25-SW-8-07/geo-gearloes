#![cfg(test)]
#![allow(dead_code)]

// #![cfg_attr(test)]
use std::collections::HashMap;
use std::fmt::Write;
use std::{borrow::Cow, collections::HashSet};

use geo_types::{Line, LineString, Point};
use rstar::{primitives::GeomWithData, PointDistance};
use rusty_roads::{Id, NearestNeighbor, RoadIndex};
use sqlx::Pool;
use sqlx::{pool::PoolConnection, Postgres};

// use crate::wkb_to_linestring;
use ::atlas::wkb_to_linestring;
use location_obfuscation::*;

use rayon::prelude::*;
type Trajectory = (i32, LineString);

#[derive(Clone)]
struct Roads {
    ids: Vec<Id>,
    roads: Vec<LineString<f64>>,
}
impl NearestNeighbor<Point, LineString<f64>> for Roads {
    fn nearest_neighbor(&self, point: Point) -> Option<GeomWithData<LineString<f64>, u64>> {
        let data = self.ids.iter().zip(self.roads.iter()).fold(None, |acc, x| {
            let distance = x.1.points().fold(None, |acc: Option<f64>, element: Point| {
                Some(acc.map_or(point.distance_2(&element), |a| {
                    a.min(point.distance_2(&element))
                }))
            });
            if distance.is_none() {
                return acc;
            }

            if acc.is_none() {
                return Some((x.0, x.1, distance.unwrap()));
            }

            if distance.unwrap() < acc.unwrap().2 {
                return Some((x.0, x.1, distance.unwrap()));
            }

            acc
        });

        let something = data.map_or(None, |(id, line, _)| {
            Some(GeomWithData::<LineString<f64>, u64>::new(line.clone(), *id))
        });

        return something;
    }

    fn nearest_neighbor_road(&self, point: Point<f64>, id: Id) -> Option<Point> {
        let id = self.ids.iter().position(|x| *x == id).unwrap();
        self.roads[id as usize]
            .points()
            .fold(None, |acc, x| {
                if acc.is_none() {
                    return Some(x);
                }
                let distance = point.distance_2(&x);
                if point.distance_2(&acc.unwrap()) > distance {
                    return Some(x);
                } else {
                    return acc;
                }
            })
            .map(|x| geo_types::Point::from(x))
    }
}

// impl NearestNeighbor<Point, LineString<f64>> for RoadIndex{
//     fn nearest_neighbor(&self, point: Point) -> Option<GeomWithData<LineString<f64>, Id>> {

//         todo!()
//     }

//     fn nearest_neighbor_road(&self, point: Point, id: Id) -> Option<Point> {
//         todo!()
//     }
// }

pub async fn map_match_porto(
    pool: Pool<Postgres>,
    roadnetwork: &Roads,
    porto_pool: Pool<Postgres>,
) -> Result<(), sqlx::Error>
where
{
    const CHUNK_SIZE: i32 = 100; //TODO: should be large if road network index is used
                                 // println!("test");
    const _: () = assert!(CHUNK_SIZE > 0);
    let ids: Vec<(i32,)> = sqlx::query_as("select id from taxadata;")
        .fetch_all(&porto_pool)
        .await?;
    let ids = ids.into_iter().map(|e| e.0);

    let mut received = HashMap::<i32, LineString>::with_capacity(ids.len());
    let all_ids: HashSet<i32> = HashSet::from_iter(ids);
    let mut keys: HashSet<i32> = HashSet::with_capacity(all_ids.len());

    while all_ids != keys {
        let (ids, trajs): (Vec<i32>, Vec<LineString>) = get_trajectories(
            porto_pool.acquire().await?,
            CHUNK_SIZE,
            Some(Vec::from_iter(keys.iter().copied()).as_slice()),
        )
        .await?
        .into_iter()
        .unzip();

        keys = keys
            .union(&HashSet::from_iter(ids.iter().copied()))
            .copied()
            .collect();
        received.extend(ids.iter().copied().zip(trajs.clone().into_iter()));
        let matched = map_match(
            &ids.iter().copied().zip(trajs).collect::<Vec<Trajectory>>(),
            roadnetwork,
        );

        let _ = insert_matched_trajectories(porto_pool.acquire().await.unwrap(), &matched)
            .await
            .expect("failed to insert matched trajectories");
    }

    Ok(())
}

async fn insert_matched_trajectories(
    mut conn: PoolConnection<Postgres>,
    trajs: &[Trajectory],
) -> Result<(), sqlx::Error> {
    let (ids, trajs): (Vec<i32>, Vec<Vec<u8>>) = trajs
        .into_iter()
        .filter_map(|(id, ls)| {
            let mut buffer: Vec<u8> = vec![];
            let a = wkb::writer::write_line_string(&mut buffer, ls, wkb::Endianness::LittleEndian)
                .ok()?;
            Some((*id, buffer))
        })
        .unzip();

    let sql = format!("insert into matched_taxa (tid, geom) select tid, geom from unnest($1) as tid, unnest($2) as geom ON CONFLICT DO NOTHING;"); //? should probably update conflict rows instead of doing nothing, but its a test and i cant be bothered
    let insert: Option<()> = sqlx::query_as(&sql)
        .bind(ids)
        .bind(&trajs)
        .fetch_optional(&mut *conn)
        .await?;

    Ok(insert.unwrap_or(())) //TIHI
}

fn map_match(trajs: &[Trajectory], roadnetwork: &Roads) -> Vec<Trajectory> {
    let matched = trajs
        .par_iter()
        .filter_map(|(id, traj)| {
            Some((
                *id,
                obfuscate_points(
                    traj.points().collect::<Vec<Point>>().into_iter(),
                    roadnetwork,
                )
                .ok()?,
            ))
        })
        .map(|(id, ps)| (id, LineString::from(ps)));

    matched.collect()
}

async fn get_trajectories(
    mut conn: PoolConnection<Postgres>,
    chunk_size: i32,
    exclude: Option<&[i32]>,
) -> Result<Vec<(i32, LineString)>, sqlx::Error> {
    let exclude = exclude.unwrap_or(&[]);
    let where_clause = match exclude.len() {
        0 => "".into(),
        _ => {
            let not_in = exclude.iter().fold(String::new(), |mut acc, &id| {
                let _ = write!(acc, "{id},");
                acc
            });
            let not_in = &not_in[0..not_in.len() - 1];
            format!("where id not in ({not_in})")
        }
    };
    let sql = format!(
        "select id, st_asbinary(trajectory::geometry,'NDR') from taxadata {where_clause} limit {chunk_size};"
    );

    let trajectories: Vec<(i32, Vec<u8>)> = sqlx::query_as(&sql).fetch_all(&mut *conn).await?;
    let trajectories: Vec<(i32, LineString)> = trajectories
        .into_iter()
        .filter_map(|(id, bytea)| {
            Some((id, wkb_to_linestring(&bytea)?)) //TODO: do something about invalid linestrings
        })
        .collect();
    Ok(trajectories)
}

mod tests {

    use ::atlas::*;
    use dotenvy::dotenv;
    use geo_types::LineString;
    use rusty_roads::RoadIndex;
    use sqlx::{Pool, Postgres};
    use std::env;
    use std::sync::LazyLock;

    use crate::{insert_matched_trajectories, map_match, map_match_porto, Roads, Trajectory};
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

    #[ignore = "this test is very time consuming"]
    #[async_std::test]
    async fn match_test() {
        const PORTUGAL: Bbox<f64> = ((-9.8282947, 42.461873), (-6.4709611, 36.4666192));
        const PORTUGAL_ROAD_COUNT: i32 = 1_362_233;
        println!("starting match test");
        dotenv().unwrap();
        let username = env::var("DB_PROGRAMUSER").unwrap();
        let db_password = env::var("DB_PROGRAMPASSWORD").unwrap();
        let porto_db = env::var("DB_TAXA").unwrap();
        let porto_conn = format!(
            "postgres://{}:{}@{}/{}",
            &username, &db_password, &*ADDRESS, &porto_db
        );
        let porto_conn = create_pool(&porto_conn, Some(100)).await.expect("msg");

        let conn = (*POOL)
            .acquire()
            .await
            .expect("failed to establish database conenction");
        let roads = box_query(conn, PORTUGAL, Some(PORTUGAL_ROAD_COUNT as u32))
            .await
            .expect("failed to get portugese roads"); // can take ~25 seconds

        debug_assert_eq!(
            roads.len() as i32,
            PORTUGAL_ROAD_COUNT,
            "expected {PORTUGAL_ROAD_COUNT} roads, got {}",
            roads.len()
        );
        let road_network: (Vec<_>, Vec<_>) = roads.into_iter().map(|r| (r.id, r.geom)).unzip();
        let road_network = Roads {
            ids: road_network.0,
            roads: road_network.1,
        };

        let index = RoadIndex::from(&road_network.ids, &road_network.roads); // !hov hov

        map_match_porto((*POOL).clone(), &road_network, porto_conn)
            .await
            .expect("asdaa");

    }
}
