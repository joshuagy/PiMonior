use anyhow::Result;
use tempfile::Builder;
use std::time::{SystemTime};
use prometheus_client::encoding::{EncodeLabelSet, EncodeLabelValue};
use prometheus_client::encoding::text::encode;
use prometheus_client::metrics::family::Family;
use prometheus_client::registry::Registry;
use prometheus_client::metrics::gauge::Gauge;
use std::sync::{Arc, Mutex};
use std::sync::atomic::AtomicU64;
use std::thread;
use std::thread::sleep;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};

static DEBIT:f64 = 0.0;
#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelValue)]
pub enum Method {
    Get
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct RequestLabels {
    pub method: Method,
}

pub struct Metrics {
    pub requests: Gauge<f64, AtomicU64>,
}

impl Metrics {
    fn new(registry: &mut Registry) -> Self {
        let requests = Gauge::default();

        registry.register(
            "download_link",
            "Download speed",
            requests.clone(),
        );

        Self { requests }
    }
}
pub struct AppState {
    pub registry: Registry,
}

async fn metrics_handler(metrics: web::Data<Arc<Metrics>>,registry: web::Data<Arc<Registry>>,) -> impl Responder {
    let mut buffer = String::new();
    encode(&mut buffer, &registry).unwrap();

    HttpResponse::Ok()
        .content_type("application/openmetrics-text; version=1.0.0; charset=utf-8")
        .body(buffer)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let mut registry = Registry::default();
    let metrics = Arc::new(Metrics::new(&mut registry));
    let registry = Arc::new(registry);

    let metrics_clone = metrics.clone();

    download(metrics_clone).await;

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(metrics.clone()))
            .app_data(web::Data::new(registry.clone()))
            .route("/metrics", web::get().to(metrics_handler))
    })
        .bind(("127.0.0.1", 8080)).expect("Failed to bind")
        .run()
        .await

    //Curl https://speed.cloudflare.com/__down?bytes=100000000 --> Get start time / get endtime = duration fore 100mb
}

async fn download(metrics: Arc<Metrics>) -> Result<()> {
    let now = SystemTime::now();

    let tmp_dir = Builder::new().prefix("example").tempdir()?;
    let target = "https://speed.cloudflare.com/__down?bytes=10000000";
    let response = reqwest::blocking::get(target)?;

    let mut dest = {
        let fname = response
            .url()
            .path_segments()
            .and_then(|segments| segments.last())
            .and_then(|name| if name.is_empty() { None } else { Some(name) })
            .unwrap_or("tmp.bin");

        println!("file to download: '{}'", fname);
        let fname = tmp_dir.path().join(fname);
        println!("will be located under: '{:?}'", fname);
    };
    let _ = response.bytes()?;
    match now.elapsed() {
        Ok(elapsed) => {
            let float: f64 = (10.0 / elapsed.as_secs_f64()) * 8.0;
            println!("Download: {float} Mb/s", );
            metrics.requests.set(float);
        }
        Err(e) => {
            println!("Back to the futur {e:?}");
        }
    }
    sleep(std::time::Duration::from_secs(60));

    Ok(())
}
