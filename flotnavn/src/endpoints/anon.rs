use std::collections::HashMap;

use actix_web::{
    dev::{ServiceFactory, ServiceRequest},
    get, web, App, Error, HttpResponse, Responder,
};
use sqlx::PgPool;

use super::get_bbox;

pub fn services<T: ServiceFactory<ServiceRequest, Config = (), Error = Error, InitError = ()>>(
    app: App<T>,
) -> App<T> {
    app.service(get_ks_in_bbox)
}

#[get("/get_ks_in_bbox.parquet")]
async fn get_ks_in_bbox(
    pool: web::Data<PgPool>,
    query: web::Query<HashMap<String, String>>,
) -> impl Responder {
    let _bbox = get_bbox(&query);

    HttpResponse::Ok()
}

