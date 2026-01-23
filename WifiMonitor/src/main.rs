use anyhow::Result;
use tempfile::Builder;
use std::time::{SystemTime};
use reqwest::Client;
use url::Url;
use prometheus_client::encoding::{EncodeLabelSet, EncodeLabelValue};
use prometheus_client::encoding::text::encode;
use prometheus_client::metrics::counter::{Atomic, Counter};
use prometheus_client::metrics::family::Family;
use prometheus_client::registry::Registry;
use std::io::Write;

#[tokio::main]
async fn main() {
    let mut registry = <Registry>::default();

    #[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
    struct Labels {
        method: Method,
        download: String,
    };

    #[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelValue)]
    enum Method {
        GET,
        PUT,
    };

    let http_requests = Family::<Labels, Counter>::default();
    registry.register(
        "http_requests",
        "Number of HTTP requests received",
        http_requests.clone(),
    );
    let now = SystemTime::now();
    tokio::task::spawn_blocking(|| download()).await;
    match now.elapsed() {
        Ok(elapsed) => {
            println!("Duration {}", elapsed.as_secs());
            let float: f32 = (100.0/elapsed.as_secs_f32())*8.0;
            println!("Download: {float} Mb/s", );
            http_requests.get_or_create(
                &Labels { method: Method::GET, download: float.to_string() }
            ).inc();
        }
        Err(e) => {
            println!("Back to the futur {e:?}");
        }
    }

    let mut buffer = String::new();
    encode(&mut buffer, &registry).unwrap();

    let expected = "# HELP http_requests Number of HTTP requests received.\n".to_owned() +
        "# TYPE http_requests counter\n" +
        "http_requests_total{method=\"GET\",path=\"/metrics\"} 1\n" +
        "# EOF\n";
    assert_eq!(expected, buffer);
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
