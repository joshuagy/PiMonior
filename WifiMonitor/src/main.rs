use anyhow::Result;
use tempfile::Builder;
use std::time::{SystemTime};
use reqwest::Client;
use url::Url;
use prometheus_client::encoding::{EncodeLabelSet, EncodeLabelValue};
use prometheus_client::encoding::text::encode;
use prometheus_client::metrics::counter::{ Counter};
use prometheus_client::metrics::family::Family;
use prometheus_client::registry::Registry;
use std::io::Write;
use std::sync::Mutex;
use actix_web::middleware::Compress;
use actix_web::{web, App, HttpResponse, HttpServer, Responder, Result as ActixResult};

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelValue)]
pub enum Method {
    Get,
    Post,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct MethodLabels {
    pub method: Method,
}

pub struct Metrics {
    requests: Family<MethodLabels, Counter>,
}

impl Metrics {
    pub fn update_metrics(&self, method: Method) {
        self.requests.get_or_create(&MethodLabels { method }).; //TODO Set download data to metrics -> Learn OpenMetrics process
    }
}

pub struct AppState {
    pub registry: Registry,
}

pub async fn metrics_handler(state: web::Data<Mutex<AppState>>) -> actix_web::Result<HttpResponse> {
    let state = state.lock().unwrap();
    let mut body = String::new();
    encode(&mut body, &state.registry).unwrap();
    Ok(HttpResponse::Ok()
        .content_type("application/openmetrics-text; version=1.0.0; charset=utf-8")
        .body(body))
}

#[tokio::main]
async fn main() {
    let now = SystemTime::now();
    tokio::task::spawn_blocking(|| download()).await;
    match now.elapsed() {
        Ok(elapsed) => {
            let float: f32 = (100.0/elapsed.as_secs_f32())*8.0;
            println!("Download: {float} Mb/s", );
                    }
        Err(e) => {
            println!("Back to the futur {e:?}");
        }
    }
    let metrics = web::Data::new(Metrics {
        requests: Family::default(),
    });
    let mut state = AppState {
        registry: Registry::default(),
    };
    state
        .registry
        .register("download_link", "Download speed", metrics.requests.clone());
    let state = web::Data::new(Mutex::new(state));

    HttpServer::new(move || {
        App::new()
            .wrap(Compress::default())
            .app_data(metrics.clone())
            .app_data(state.clone())
            .service(web::resource("/metrics").route(web::get().to(metrics_handler)))
    })
        .bind(("127.0.0.1", 8080)).expect("Failed to bind")
        .run()
        .await.expect("Http server error");
    //Curl https://speed.cloudflare.com/__down?bytes=100000000 --> Get start time / get endtime = duration fore 100mb
}
fn download() -> Result<()> {
    let tmp_dir = Builder::new().prefix("example").tempdir()?;
    let target = "https://speed.cloudflare.com/__down?bytes=100000000";
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
    let content =  response.bytes()?;
    Ok(())
}
