use std::sync::Arc;

use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use prometheus_client::encoding::text::encode;
use prometheus_client::metrics::counter::Counter;
use prometheus_client::metrics::family::Family;
use prometheus_client::registry::Registry;
use prometheus_client::encoding::{EncodeLabelSet, EncodeLabelValue};

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelValue)]
enum HttpMethod {
    Get,
    Post,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
struct RequestLabels {
    method: HttpMethod,
}

struct Metrics {
    requests_total: Family<RequestLabels, Counter>,
}

impl Metrics {
    fn new(registry: &mut Registry) -> Self {
        let requests_total = Family::default();

        registry.register(
            "http_requests_total",
            "Total number of HTTP requests",
            requests_total.clone(),
        );

        Self { requests_total }
    }

    fn inc_requests(&self, method: HttpMethod) {
        self.requests_total
            .get_or_create(&RequestLabels { method })
            .inc();
    }
}

async fn metrics_handler(
    metrics: web::Data<Arc<Metrics>>,
    registry: web::Data<Arc<Registry>>,
) -> impl Responder {
    let mut buffer = String::new();
    encode(&mut buffer, &registry).unwrap();

    HttpResponse::Ok()
        .content_type("application/openmetrics-text; version=1.0.0; charset=utf-8")
        .body(buffer)
}

async fn hello_handler(metrics: web::Data<Arc<Metrics>>) -> impl Responder {
    metrics.inc_requests(HttpMethod::Get);
    "hello"
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let mut registry = Registry::default();
    let metrics = Arc::new(Metrics::new(&mut registry));

    let registry = Arc::new(registry);

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(metrics.clone()))
            .app_data(web::Data::new(registry.clone()))
            .route("/", web::get().to(hello_handler))
            .route("/metrics", web::get().to(metrics_handler))
    })
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
}
