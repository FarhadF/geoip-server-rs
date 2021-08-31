use std::ffi::OsStr;
use std::fs;
use std::net::IpAddr;
use std::str::FromStr;
use std::sync::Arc;

use actix_web::*;
use anyhow::{anyhow, Result};
use async_std::io::ErrorKind;
use clap::{App as clapApp, Arg};
use flate2::read::GzDecoder;
use maxminddb::{MaxMindDBError, Reader};
use reqwest::header::{HeaderValue, CONTENT_LENGTH, RANGE};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use slog::Drain;
use slog::*;
use slog_async::Async;
use std::io::copy;
use tar::Archive;

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

#[derive(Serialize)]
struct GeoResponse<'a> {
    pub ip: &'a str,
    pub country_code: &'a str,
    pub country_name: &'a str,
    pub continent: &'a str,
    pub continent_code: &'a str,
    pub region_code: &'a str,
    pub region_name: &'a str,
    pub city: &'a str,
    pub zip_code: &'a str,
    pub time_zone: &'a str,
    pub latitude: &'a f64,
    pub longitude: &'a f64,
    pub metro_code: &'a u16,
}

#[get("/health-check")]
async fn health_check() -> impl Responder {
    HttpResponse::Ok().json(&HealthResponse { status: "ok" })
}

#[derive(Serialize, Deserialize, Debug)]
struct ResponseError {
    error: String,
}

struct AppState {
    reader: Arc<Reader<Vec<u8>>>,
}

struct PartialRangeIter {
    start: u64,
    end: u64,
    buffer_size: u32,
}

impl PartialRangeIter {
    pub fn new(start: u64, end: u64, buffer_size: u32) -> Result<Self> {
        if buffer_size == 0 {
            return Err(anyhow!(
                "invalid buffer_size, give a value greater than zero."
            ));
        }
        Ok(PartialRangeIter {
            start,
            end,
            buffer_size,
        })
    }
}

impl Iterator for PartialRangeIter {
    type Item = HeaderValue;
    fn next(&mut self) -> Option<Self::Item> {
        if self.start > self.end {
            None
        } else {
            let prev_start = self.start;
            self.start += std::cmp::min(self.buffer_size as u64, self.end - self.start + 1);
            Some(
                HeaderValue::from_str(&format!("bytes={}-{}", prev_start, self.start - 1))
                    .expect("string provided by format!"),
            )
        }
    }
}

#[get("/geoip/json/{ip}")]
async fn get_ip(ip: web::Path<String>, state: web::Data<AppState>) -> impl Responder {
    let ip: core::result::Result<IpAddr, std::net::AddrParseError> = (ip).parse();
    let ip: IpAddr = match ip {
        Ok(ip) => ip,
        Err(e) => {
            return HttpResponse::InternalServerError().json(error_factory(e.to_string()));
        }
    };
    let city: core::result::Result<maxminddb::geoip2::City, MaxMindDBError> =
        state.reader.lookup(ip);
    let city = match city {
        Ok(city) => city,
        Err(e) => {
            return HttpResponse::InternalServerError().json(error_factory(e.to_string()));
        }
    };
    let region = city
        .subdivisions
        .as_ref()
        .filter(|subdivs| !subdivs.is_empty())
        .and_then(|subdivs| subdivs.get(0));

    let resp = GeoResponse {
        ip: &ip.to_string(),
        country_code: city
            .country
            .as_ref()
            .and_then(|country| country.iso_code)
            .unwrap_or(""),
        country_name: city
            .country
            .as_ref()
            .and_then(|country| country.names.as_ref())
            .and_then(|names| names.get("en"))
            .unwrap_or(&""),
        continent: city
            .continent
            .as_ref()
            .and_then(|cont| cont.names.as_ref())
            .and_then(|names| names.get("en"))
            .unwrap_or(&""),
        continent_code: city
            .continent
            .as_ref()
            .and_then(|cont| cont.code)
            .unwrap_or(""),
        region_code: region
            .and_then(|subdiv| subdiv.iso_code)
            .unwrap_or(""),
        region_name: region
            .and_then(|subdiv| subdiv.names.as_ref())
            .and_then(|names| names.get("en"))
            .unwrap_or(&""),
        city: city
            .city
            .as_ref()
            .and_then(|city| city.names.as_ref())
            .and_then(|names| names.get("en"))
            .unwrap_or(&""),
        zip_code: city
            .postal
            .as_ref()
            .and_then(|postal| postal.code)
            .unwrap_or(""),
        time_zone: city
            .location
            .as_ref()
            .and_then(|loc| loc.time_zone)
            .unwrap_or(""),
        latitude: city
            .location
            .as_ref()
            .and_then(|loc| loc.longitude.as_ref())
            .unwrap_or(&0.0),
        longitude: city
            .location
            .as_ref()
            .and_then(|loc| loc.latitude.as_ref())
            .unwrap_or(&0.0),
        metro_code: city
            .location.
            as_ref().
            and_then(|loc| loc.metro_code.as_ref())
            .unwrap_or(&0),
    };
    HttpResponse::Ok().json2(&resp)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let matches = clapApp::new("Geoip Server")
        .version("1.0")
        .author("Farhad Farahi <farhad.farahi@gmail.com>")
        .about("geoip server")
        .arg(
            Arg::with_name("license")
                .short("l")
                .long("license")
                .value_name("license")
                .help("maxmind license (its free, but you have to sign up)")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("logtype")
                .short("t")
                .long("logtype")
                .value_name("logtype")
                .help("sets the logging type [json/terminal]")
                .required(false)
                .takes_value(true)
                .default_value("terminal"),
        )
        .arg(
            Arg::with_name("address")
                .short("a")
                .long("address")
                .value_name("address")
                .help("ip address to bind to")
                .required(false)
                .takes_value(true)
                .default_value("0.0.0.0"),
        )
        .arg(
            Arg::with_name("port")
                .short("p")
                .long("port")
                .value_name("port")
                .help("http service port")
                .required(false)
                .takes_value(true)
                .default_value("8080"),
        )
        .get_matches();
    let license = matches.value_of("license").unwrap();
    let log_type = matches.value_of("logtype").unwrap();
    let address = matches.value_of("address").unwrap();
    let port = matches.value_of("port").unwrap();
    let drain: Fuse<Async>;
    if log_type == "terminal" {
        let decorator = slog_term::TermDecorator::new().build();
        let d = slog_term::FullFormat::new(decorator).build().fuse();
        drain = slog_async::Async::new(d).build().fuse();
    } else {
        let d = slog_json::Json::new(std::io::stdout())
            .set_pretty(false)
            .add_default_keys()
            .build()
            .fuse();
        drain = slog_async::Async::new(d).build().fuse();
    }
    let log = slog::Logger::root(
        drain,
        o!("location" => PushFnValue(|r: &Record, ser: PushFnValueSerializer| {
            ser.emit(format_args!("{}:{}", r.file(), r.line()))
        })),
    );
    info!(log, "{}", "Downloading the DB");
    let result = download(license).await;
    if let Err(e) = result {
        error!(log, "{:?}", e);
        return Err(std::io::Error::new(ErrorKind::Other, e.to_string()));
    }
    info!(log, "{}", "extracting the archive");
    let result = extract(log.clone()).await;
    if let Err(e) = result {
        error!(log, "{:?}", e);
        return Err(std::io::Error::new(ErrorKind::Other, e.to_string()));
    }
    let reader = maxminddb::Reader::open_readfile("GeoLite2-City.mmdb").unwrap();
    let r = Arc::new(reader);
    info!(log, "server starting");
    let server = HttpServer::new(move || {
        App::new()
            .data(AppState {
                reader: r.clone(),
            })
            .service(get_ip)
            .service(health_check)
    });
    server.bind(format!("{}:{}", address, port))?
        .run()
        .await
}

fn error_factory(e: String) -> ResponseError {
    ResponseError { error: e }
}

async fn download(license: &str) -> Result<()> {
    let url = &format!("https://download.maxmind.com/app/geoip_download?edition_id=GeoLite2-City&license_key={}&suffix=tar.gz", license);
    const CHUNK_SIZE: u32 = 10485760;
    let client = reqwest::Client::builder().build()?;
    let response = client.head(url).send().await?;
    let length = response.headers().get(CONTENT_LENGTH);
    let length = length.unwrap().to_str()?;
    let length = u64::from_str(length)?;
    let mut output_file = fs::File::create("temparchive")?;
    let partial_range = PartialRangeIter::new(0, length - 1, CHUNK_SIZE)?;
    for range in partial_range {
        let response = client.get(url).header(RANGE, range).send().await?;

        let status = response.status();
        if !(status == StatusCode::OK || status == StatusCode::PARTIAL_CONTENT) {
            return Err(anyhow!("Unexpected server response: {}", status));
        }
        let content = response.bytes().await?;
        copy(&mut content.as_ref(), &mut output_file)?;
    }
    Ok(())
}

async fn extract(log: Logger) -> Result<()> {
    let path = "temparchive";
    let tar_gz = fs::File::open(path)?;
    let tar = GzDecoder::new(tar_gz);
    let mut archive = Archive::new(tar);
    let entries = archive.entries()?;
    for entry in entries {
        let mut entry = entry?;
        let path = entry.path()?;
        let ext = path.extension();
        let os_str = OsStr::new("mmdb");
        if ext == Some(os_str) {
            let p = path.to_str();
            let p = match p {
                Some(x) => x,
                None => return Err(anyhow!("path to string conversion failed")),
            };
            let mut split = p.split('/');
            let first = split.next().unwrap();
            let prefix = path.strip_prefix(first.to_owned() + "/")?.to_owned();
            entry.unpack(prefix)?;
            fs::remove_file("temparchive")?;
            info!(log, "found the db");
        }
    }
    Ok(())
}
