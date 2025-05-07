use std::collections::HashMap;

use actix_web::{
    dev::{ServiceFactory, ServiceRequest},
    get, post, web, App, Error, HttpResponse, Responder,
};
use sqlx::PgPool;

use atlas::anonymity;
use comms::Parquet;

use super::get_bbox;

pub fn services<T: ServiceFactory<ServiceRequest, Config = (), Error = Error, InitError = ()>>(
    app: App<T>,
) -> App<T> {
    app.service(get_ks_in_bbox)
        .service(post_trajectory)
        .service(add_unknown_visit)
}

#[get("/get_ks_in_bbox.parquet")]
async fn get_ks_in_bbox(
    pool: web::Data<PgPool>,
    query: web::Query<HashMap<String, String>>,
) -> impl Responder {
    let bbox = get_bbox(&query);
    let conn = match pool.acquire().await {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().body(""),
    };

    let anon_data = match anonymity::box_anonymity_query(conn, bbox, None).await {
        Ok(v) => v,
        Err(_) => return HttpResponse::InternalServerError().body(""),
    };

    let data = match anon_data.to_parquet() {
        Ok(d) => d,
        Err(_) => return HttpResponse::InternalServerError().body(""),
    };

    HttpResponse::Ok().body(data)
}

#[post("/add_trajectory")]
async fn post_trajectory(pool: web::Data<PgPool>, payload: web::Bytes) -> impl Responder {
    let mut conn = match pool.acquire().await {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().body(""),
    };

    let unpacked_traj: rusty_roads::Trajectories = match Parquet::from_parquet(payload) {
        Ok(v) => v,
        Err(_) => {
            return HttpResponse::BadRequest().body("The provided data is not trajectories");
        }
    };

    let buffer: Vec<Vec<u8>> = match unpacked_traj
        .geom
        .iter()
        .map(|g| {
            let mut buff = Vec::new();

            match wkb::writer::write_line_string(&mut buff, g, wkb::Endianness::LittleEndian) {
                Ok(_) => Ok(buff),
                Err(_) => Err(()),
            }
        })
        .collect::<Result<Vec<Vec<u8>>, _>>()
    {
        Ok(b) => b,
        Err(_) => return HttpResponse::BadRequest().body("Could not add trajectory to database."),
    };

    match atlas::anonymity::add_trajectories(conn, buffer).await {
        Ok(_) => (),
        Err(_) => return HttpResponse::InternalServerError().body("Interanl server error"),
    };

    HttpResponse::Ok().body("")
}

#[post("/add_unknown_visit")]
async fn add_unknown_visit(
    pool: web::Data<PgPool>,
    query: web::Query<HashMap<String, String>>,
) -> impl Responder {
    let bbox = get_bbox(&query);
    let conn = match pool.acquire().await {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().body(""),
    };

    let probability = match query.0.get("probability") {
        Some(v) => v,
        None => return HttpResponse::BadRequest().body("No probability provided"),
    };

    let probability: f64 = match probability.parse() {
        Ok(v) => v,
        Err(_) => return HttpResponse::BadRequest().body("probability value is not of type float"),
    };

    match atlas::anonymity::box_add_unknownvisits(conn, bbox, anonymity::Probability(probability))
        .await
   {
        Ok(_) => (),
        Err(_) => return HttpResponse::InternalServerError().body("Internal server error"),
    };

    HttpResponse::Ok().body("")
}
