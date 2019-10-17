use crate::config::ServerConfig;
use crate::scraper::metrics::{Sample, TimeSeries};
use actix_files as fs;
use actix_web::dev::Server;
use actix_web::http::StatusCode;
use actix_web::{get, guard, middleware, web, App, HttpResponse, HttpServer};
use metrics_core::{Builder, Drain, Observe};
use metrics_runtime::observers::PrometheusBuilder;
use metrics_runtime::Controller;
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

/// Prometheus scraping endpoint
#[get("/prometheus")]
fn prometheus(state: web::Data<Controller>) -> HttpResponse {
    let mut observer = PrometheusBuilder::new().build();
    state.observe(&mut observer);
    HttpResponse::build(StatusCode::OK).body(observer.drain())
}

#[derive(Debug, Serialize)]
struct Series<'a> {
    topic: &'a str,
    data: Vec<Sample>,
}

#[get("/metrics")]
fn time_series(state: web::Data<Vec<Arc<TimeSeries>>>) -> HttpResponse {
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

pub fn run(
    config: ServerConfig,
    ts: Vec<Arc<TimeSeries>>,
    scraper_metrics: Controller,
) -> std::io::Result<Server> {
    let ts = web::Data::new(ts);
    let scraper_metrics = web::Data::new(scraper_metrics);
    let create_server = move || {
        App::new()
            .register_data(ts.clone())
            .register_data(scraper_metrics.clone())
            .wrap(middleware::Logger::default())
            .service(index)
            .service(time_series)
            .service(prometheus)
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
