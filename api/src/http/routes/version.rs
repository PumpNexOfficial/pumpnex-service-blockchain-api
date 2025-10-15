/// Version route

use actix_web::{web, HttpResponse, Responder};
use serde::Serialize;

use crate::app_state::AppState;

#[derive(Serialize)]
struct VersionResponse {
    name: String,
    version: String,
}

pub async fn version(state: web::Data<AppState>) -> impl Responder {
    HttpResponse::Ok().json(VersionResponse {
        name: state.service_config.name.clone(),
        version: state.service_config.version.clone(),
    })
}
