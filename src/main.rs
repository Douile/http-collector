use std::fs;
use std::io::prelude::*;
use std::io::{BufReader, BufWriter};
use std::net::{IpAddr, SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::thread;

use clap::{App, Arg};

mod error;
mod http;

use http::parse_http;

#[derive(Debug)]
struct Config {
    output_dir: Option<String>,
    counter: Arc<Mutex<u32>>,
    json: bool,
}

fn handle_client(stream: &TcpStream, config: &Config) -> std::io::Result<()> {
    let ip = format!("{}", stream.peer_addr()?.ip());

    let mut reader = BufReader::new(stream);
    let mut writer = BufWriter::new(stream);

    let mut buf = Vec::default();
    let res_type = if let Ok(http) = parse_http(&mut reader, &mut buf) {
        if config.json {
            println!(
                "{}",
                serde_json::to_string(&serde_json::json!({
                    "type": "http",
                    "peer": ip,
                    "data": http,
                }))?
            );
        } else {
            println!("[{}] {:?}", ip, http);
        }

        write!(writer, "HTTP/1.1 204 No Content\r\n\r\n")?;

        "http"
    } else {
        reader.read_to_end(&mut buf)?;
        if config.json {
            println!(
                "{}",
                serde_json::to_string(&serde_json::json!({
                    "type": "raw",
                    "peer": ip,
                    "data": buf,
                }))?
            );
        } else {
            println!("[{}] {:?}", ip, String::from_utf8_lossy(&buf));
        }

        "raw"
    };

    if let Some(output_dir) = &config.output_dir {
        let mut counter = config.counter.lock().unwrap();
        *counter += 1;
        let path: PathBuf = [output_dir, &format!("{:05}-{}-{}", counter, ip, res_type)]
            .iter()
            .collect();
        fs::write(path, buf)?;
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new("http-collector")
        .about("Log incoming TCP/HTTP traffic easily and completely")
        .arg(
            Arg::with_name("port")
                .short("p")
                .long("port")
                .help("Port to bind")
                .takes_value(true)
                .default_value("8000"),
        )
        .arg(
            Arg::with_name("addr")
                .short("a")
                .long("address")
                .help("Address to bind")
                .takes_value(true)
                .default_value("0.0.0.0"),
        )
        .arg(
            Arg::with_name("output_dir")
                .short("d")
                .long("output-dir")
                .help("Directory to output raw connections to")
                .takes_value(true),
        );

    if cfg!(feature = "json") {
        app = app.arg(
            Arg::with_name("json")
                .short("j")
                .long("json")
                .help("Output connections as json"),
        );
    }

    let matches = app.get_matches();

    let port: u16 = matches.value_of("port").unwrap().parse()?;
    let ip: &str = matches.value_of("addr").unwrap();
    let output_dir = matches.value_of("output_dir");
    if let Some(output_dir) = output_dir {
        match fs::metadata(output_dir) {
            Ok(attr) => {
                if !attr.is_dir() {
                    Err("Output directory must be a directory")?;
                }
            }
            Err(_) => {
                fs::create_dir(output_dir)?;
            }
        }
    }

    let config = Arc::new(Config {
        output_dir: output_dir.map(|s| s.to_string()),
        counter: Arc::new(Mutex::new(0)),
        #[cfg(feature = "json")]
        json: matches.is_present("json"),
        #[cfg(not(feature = "json"))]
        json: false,
    });

    let addr: SocketAddr = SocketAddr::new(IpAddr::from_str(ip)?, port);

    eprintln!("Listening on {:?}", addr);

    let listener = TcpListener::bind(addr)?;

    for connection in listener.incoming() {
        let connection = connection?;
        let config = Arc::clone(&config);
        thread::spawn(move || handle_client(&connection, &config));
    }

    Ok(())
}
