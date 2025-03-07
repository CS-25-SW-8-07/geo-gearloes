use sqlx::{pool::PoolConnection, Postgres};
// JOin roads and featureclass, returns road_id, fid and fclass
pub async fn join_roads_featureclass(
    mut conn: PoolConnection<Postgres>,
    limit: Option<u32>,
    with: &[usize],
) -> Result<Vec<(i32, i16, String)>, sqlx::Error> {
    let limit = limit.map(|x| format!("limit {x}")).unwrap_or("".into());

    let where_clause = match with.len() {
        0 => "".into(),
        _ => {
            let where_in = with
                .iter()
                .map(|id| id.to_string() + ",")
                .collect::<String>();
            let where_in = &where_in[0..where_in.len() - 1];
            format!("WHERE id IN ({where_in})")
        }
    };

    let query = format!(
        "SELECT id, fid, fclass FROM roads JOIN featureclass ON code = fid {where_clause} {limit}"
    );

    let res: Vec<(i32, i16, String)> = sqlx::query_as(&query).fetch_all(&mut *conn).await?;

    Ok(res)
}

// Join roads and roadnames, returns road_id and road_name
pub async fn join_roads_roadsnames(
    mut conn: PoolConnection<Postgres>,
    limit: Option<u32>,
) -> Result<Vec<(i32, String)>, sqlx::Error> {
    let limit = limit.map(|x| format!("limit {x}")).unwrap_or("".into());

    let query =
        format!("SELECT id, name FROM roads JOIN roadname ON roads.id = roadname.nid {limit}");

    let res: Vec<(i32, String)> = sqlx::query_as(&query).fetch_all(&mut *conn).await?;

    Ok(res)
}

// Join refmany with ref, returns road_id, rid and ref
pub async fn join_refmany_ref(
    mut conn: PoolConnection<Postgres>,
    limit: Option<u32>,
    with: &[usize],
) -> Result<Vec<(i32, i64, String)>, sqlx::Error> {
    let limit = limit.map(|x| format!("limit {x}")).unwrap_or("".into());

    let where_clause = match with.len() {
        0 => "".into(),
        _ => {
            let where_in = with
                .iter()
                .map(|id|id.to_string() + ",")
                .collect::<String>();
            let where_in = &where_in[0..where_in.len() - 1];
            format!("WHERE road_id IN ({where_in})")
        }
    };

    let query = format!(
        "SELECT road_id, ref_id, ref FROM refmany JOIN ref ON refmany.ref_id = ref.rid {where_clause} {limit}"
    );

    let res: Vec<(i32, i64, String)> = sqlx::query_as(&query).fetch_all(&mut *conn).await?;

    Ok(res)
}

// Join roads, refmany and ref, returns road_id, rid and ref
pub async fn join_roads_refmany_ref(
    mut conn: PoolConnection<Postgres>,
    limit: Option<u32>,
    with: &[usize],
) -> Result<Vec<(i32, i64, String)>, sqlx::Error> {
    let limit = limit.map(|x| format!("limit {x}")).unwrap_or("".into());

    let where_clause = match with.len() {
        0 => "".into(),
        _ => {
            let where_in = with
                .iter()
                .map(|id| id.to_string() + ",")
                .collect::<String>();
            let where_in = &where_in[0..where_in.len() - 1];
            format!("WHERE road_id IN ({where_in})")
        }
    };

    let query = format!(
        "SELECT road_id, rid, ref FROM ref JOIN refmany ON refmany.ref_id = ref.rid JOIN roads ON refmany.road_id = roads.id {where_clause} {limit}"
    );

    let res: Vec<(i32, i64, String)> = sqlx::query_as(&query).fetch_all(&mut *conn).await?;

    Ok(res)
}

#[cfg(test)]
mod tests {
    use super::super::bind;
    use super::*;
    use dotenvy::dotenv;
    use std::env;
    use std::sync::LazyLock;
    use sqlx::pool::Pool;
    
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

    mod test_of_join_roads_featureclass {

        use super::*;

        #[async_std::test]
        async fn join_no_limit_no_where() {
            let conn = (*POOL).acquire().await.expect("msg");
            assert_eq!(
                join_roads_featureclass(conn, None, &[])
                    .await
                    .expect("Paniced!")
                    .len(),
                1264962
            );
        }

        #[async_std::test]
        async fn join_limit_no_where() {
            let conn = (*POOL).acquire().await.expect("msg");
            assert_eq!(
                join_roads_featureclass(conn, Some(5), &[])
                    .await
                    .expect("Paniced!")
                    .len(),
                5
            );
        }

        #[async_std::test]
        async fn join_no_limit_with_where() {
            let conn = (*POOL).acquire().await.expect("msg");

            assert_eq!(
                join_roads_featureclass(conn, None, &[64285, 64286])
                    .await
                    .expect("Paniced!")
                    .len(),
                2
            );
        }

        #[async_std::test]
        async fn join_with_limit_with_where() {
            let conn = (*POOL).acquire().await.expect("msg");

            assert_eq!(
                join_roads_featureclass(conn, Some(1), &[64285, 64286])
                    .await
                    .expect("Paniced!")
                    .len(),
                1
            );
        }
    }

    mod test_of_join_roads_roadnames {
        use super::*;

        #[async_std::test]
        async fn join_no_limit() {
            let conn = (*POOL).acquire().await.expect("msg");
            assert_eq!(
                join_roads_roadsnames(conn, None)
                    .await
                    .expect("Paniced!")
                    .len(),
                418882
            );
        }

        #[async_std::test]
        async fn join_with_limit() {
            let conn = (*POOL).acquire().await.expect("msg");
            assert_eq!(
                join_roads_roadsnames(conn, Some(5))
                    .await
                    .expect("Paniced!")
                    .len(),
                5
            );
        }
    }

    mod test_of_join_refmany_ref {
        use super::*;

        #[async_std::test]
        async fn join_no_limit_no_where() {
            let conn = (*POOL).acquire().await.expect("msg");
            assert_eq!(
                join_refmany_ref(conn, None, &[])
                    .await
                    .expect("Paniced!")
                    .len(),
                38614
            );
        }

        #[async_std::test]
        async fn join_with_limit_no_where() {
            let conn = (*POOL).acquire().await.expect("msg");
            assert_eq!(
                join_refmany_ref(conn, Some(50), &[])
                    .await
                    .expect("Paniced!")
                    .len(),
                50
            );
        }

        #[async_std::test]
        async fn join_no_limit_with_where() {
            let conn = (*POOL).acquire().await.expect("msg");
            assert_eq!(
                join_refmany_ref(conn, None, &[51440, 500866])
                    .await
                    .expect("Paniced!")
                    .len(),
                5
            );
        }

        #[async_std::test]
        async fn join_with_limit_with_where() {
            let conn = (*POOL).acquire().await.expect("msg");
            assert_eq!(
                join_refmany_ref(conn, Some(2), &[51440, 500866])
                    .await
                    .expect("Paniced!")
                    .len(),
                2
            );
        }
    }

    mod test_of_join_roads_refmany_ref {
        use super::*;

        #[async_std::test]
        async fn join_no_limit_no_where() {
            let conn = (*POOL).acquire().await.expect("msg");
            assert_eq!(
                join_roads_refmany_ref(conn, None, &[])
                    .await
                    .expect("Paniced!")
                    .len(),
                38120
            );
        }

        #[async_std::test]
        async fn join_with_limit_no_where() {
            let conn = (*POOL).acquire().await.expect("msg");
            assert_eq!(
                join_roads_refmany_ref(conn, Some(50), &[])
                    .await
                    .expect("Paniced!")
                    .len(),
                50
            );
        }

        #[async_std::test]
        async fn join_no_limit_with_where() {
            let conn = (*POOL).acquire().await.expect("msg");
            assert_eq!(
                join_roads_refmany_ref(conn, None, &[20351, 20352])
                    .await
                    .expect("Paniced!")
                    .len(),
                3
            );
        }

        #[async_std::test]
        async fn join_with_limit_with_where() {
            let conn = (*POOL).acquire().await.expect("msg");
            assert_eq!(
                join_roads_refmany_ref(conn, Some(2), &[20351, 20352])
                    .await
                    .expect("Paniced!")
                    .len(),
                2
            );
        }
    }
}
