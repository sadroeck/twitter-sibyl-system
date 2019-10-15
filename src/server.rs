use crate::config::ServerConfig;
use actix_files as fs;
use actix_web::dev::Server;
use actix_web::http::{Method, StatusCode};
use actix_web::{get, guard, middleware, web, App, HttpResponse, HttpServer};

/// Home page
#[get("/")]
fn index() -> actix_web::Result<fs::NamedFile> {
    Ok(fs::NamedFile::open("static/index.html")?.set_status_code(StatusCode::OK))
}

/// 404 page
fn page_404() -> actix_web::Result<fs::NamedFile> {
    Ok(fs::NamedFile::open("static/404.html")?.set_status_code(StatusCode::NOT_FOUND))
}

pub fn run(config: ServerConfig) -> std::io::Result<Server> {
    let create_server = || {
        App::new()
            .wrap(middleware::Logger::default())
            .service(index)
            .service(web::resource("/metrics").route(web::get()))
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
