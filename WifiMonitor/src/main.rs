use std::process::{Command, Stdio};
use anyhow::Result;
use tempfile::Builder;
use std::time::{SystemTime};
use prometheus_client::encoding::{EncodeLabelSet, EncodeLabelValue};
use prometheus_client::encoding::text::encode;
use prometheus_client::registry::Registry;
use prometheus_client::metrics::gauge::Gauge;
use std::sync::{mpsc, Arc, Mutex};
use std::sync::atomic::AtomicU64;
use std::thread;
use std::thread::sleep;
use actix_web::{rt, web, App, HttpResponse, HttpServer, Responder};
use actix_web::dev::ServerHandle;

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
        registry.register(
            "cpu_temp",
            "CPU temp",
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

fn main(){
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let mut registry = Registry::default();
    let metrics = Arc::new(Metrics::new(&mut registry));
    let registry = Arc::new(registry);
    let (tx, rx) = mpsc::channel();

    log::info!("spawning thread for server");
    thread::spawn({
        let tx = tx.clone();          // clone du sender si nécessaire
        let metrics = Arc::clone(&metrics);
        let registry = Arc::clone(&registry);
        move || {
            let server_future = run_app(tx, metrics, registry);
            rt::System::new().block_on(server_future)
        }
    });

    let server_handle = rx.recv().unwrap();

    log::info!("spawning thread for download");
    thread::spawn({
        let metrics = Arc::clone(&metrics);
        move || {
            let server_future2 = metrics_calculator(metrics);
            rt::System::new().block_on(server_future2)
        }
    });

    loop { }
}

async fn run_app(tx: mpsc::Sender<ServerHandle>,metrics: Arc<Metrics>, registry: Arc<Registry>) -> std::io::Result<()> {
    log::info!("starting HTTP server");

    // srv is server controller type, `dev::Server`
    let server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(metrics.clone()))
            .app_data(web::Data::new(registry.clone()))
            .route("/metrics", web::get().to(metrics_handler))
    })
        .bind(("0.0.0.0", 9091))?
        .workers(2)
        .run();

    // Send server handle back to the main thread
    let _ = tx.send(server.handle());

    server.await
}


async fn metrics_calculator(metrics: Arc<Metrics>) -> Result<()>  {
    let tmp_dir = Builder::new().prefix("example").tempdir()?;
    let target = "https://speed.cloudflare.com/__down?bytes=10000000";

    loop {
        let now = SystemTime::now();
        let response = reqwest::get(target).await?;
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
        let _ = response.bytes().await?;
        match now.elapsed() {
            Ok(elapsed) => {
                let float: f64 = (10.0 / elapsed.as_secs_f64()) * 8.0;
                async { println!("Download: {float} Mb/s"); }.await;
                metrics.requests.set(float);
            }
            Err(e) => {
                println!("Back to the futur {e:?}");
            }
        }

        let output = Command::new("vcgencmd measure_temp")
            .stdout(Stdio::piped())
            .output();

        match output {
            Ok(content) => println!("{}", String::from_utf8(content.stdout).unwrap()),
            Err(e) => println!("Error: {}", e),
        }


        println!("End of gathering measurement");

        sleep(std::time::Duration::from_secs(300));
    }
}
