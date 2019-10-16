use crate::config::ServerConfig;
use crate::scraper::metrics::{Sample, TimeSeries};
use actix_files as fs;
use actix_web::dev::Server;
use actix_web::http::StatusCode;
use actix_web::{get, guard, middleware, web, App, HttpResponse, HttpServer};
use serde_derive::Serialize;
use std::sync::Arc;

/// Home page
#[get("/")]
fn index() -> actix_web::Result<fs::NamedFile> {
    Ok(fs::NamedFile::open("static/index.html")?.set_status_code(StatusCode::OK))
}

/// 404 page
fn page_404() -> actix_web::Result<fs::NamedFile> {
    Ok(fs::NamedFile::open("static/404.html")?.set_status_code(StatusCode::NOT_FOUND))
}

#[derive(Debug, Serialize)]
struct Series<'a> {
    topic: &'a str,
    data: Vec<Sample>,
}

#[get("/metrics")]
fn metrics(state: web::Data<Vec<Arc<TimeSeries>>>) -> HttpResponse {
    let data: Vec<_> = state
        .iter()
        .filter_map(|series| {
            series
                .data
                .read()
                .map(|store| store.clone())
                .ok()
                .map(|values| Series {
                    topic: series.topic.as_str(),
                    data: values,
                })
        })
        .collect();
    HttpResponse::build(StatusCode::OK).json(data)
}

pub fn run(config: ServerConfig, time_series: Vec<Arc<TimeSeries>>) -> std::io::Result<Server> {
    let time_series = web::Data::new(time_series);
    let create_server = move || {
        App::new()
            .register_data(time_series.clone())
            .wrap(middleware::Logger::default())
            .service(index)
            .service(metrics)
            .default_service(
                web::resource("")
                    // 404 for GET request
                    .route(web::get().to(page_404))
                    // all requests that are not `GET`
                    .route(
                        web::route()
                            .guard(guard::Not(guard::Get()))
                            .to(HttpResponse::MethodNotAllowed),
                    ),
            )
    };
    HttpServer::new(create_server)
        .bind(format!("{}:{}", config.host, config.port))
        .map(|server| server.start())
}
