use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::io::prelude::*;
use std::io::{BufReader, BufWriter};
use std::net::{IpAddr, SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::thread;

use clap::{App, Arg};

struct SimpleError {
    description: String,
}

impl fmt::Debug for SimpleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error: {}", self.description)
    }
}

impl fmt::Display for SimpleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error: {}", self.description)
    }
}

impl std::error::Error for SimpleError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

struct HttpStruct {
    method: String,
    path: String,
    version: f32,
    headers: HashMap<String, String>,
    content: Option<String>,
}

impl fmt::Debug for HttpStruct {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "HTTP/{} {} {} {:?}",
            self.version, self.method, self.path, self.content
        )
    }
}

struct Config {
    output_dir: Option<String>,
    counter: Arc<Mutex<u32>>,
}

fn handle_client(stream: &TcpStream, config: &Config) -> std::io::Result<()> {
    let ip = format!("{}", stream.peer_addr()?.ip());
    let mut reader = BufReader::new(stream);
    let mut writer = BufWriter::new(stream);

    let mut buf = Vec::default();
    let res_type = if let Ok(http) = parse_http(&mut reader, &mut buf) {
        println!("[{}] {:?}", ip, http);

        write!(writer, "HTTP/1.1 204 No Content\r\n\r\n")?;

        "http"
    } else {
        reader.read_to_end(&mut buf)?;
        println!("[{}] {:?}", ip, String::from_utf8_lossy(&buf));

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

fn parse_http<R: std::io::Read>(
    reader: &mut BufReader<R>,
    buf: &mut Vec<u8>,
) -> std::io::Result<HttpStruct> {
    let mut line = String::new();
    reader.read_line(&mut line)?;
    buf.extend_from_slice(line.as_bytes());

    let parts: Vec<&str> = line.split(' ').collect();
    if parts.len() != 3 {
        Err(std::io::ErrorKind::InvalidInput)?;
    }

    let method = parts[0].to_string();
    let path = parts[1].to_string();
    let version_parts: Vec<&str> = parts[2].trim().split('/').collect();
    if version_parts.len() != 2 {
        Err(std::io::ErrorKind::InvalidInput)?;
    }
    if version_parts[0] != "HTTP" {
        Err(std::io::ErrorKind::InvalidInput)?;
    }
    let version: f32 = version_parts[1]
        .parse()
        .map_err(|_| std::io::ErrorKind::InvalidInput)?;

    let mut headers: HashMap<String, String> = HashMap::new();
    loop {
        line.clear();
        reader.read_line(&mut line)?;

        buf.extend_from_slice(line.as_bytes());

        let line = line.trim();
        if line.is_empty() {
            break;
        }
        let parts: Vec<&str> = line.splitn(2, ':').collect();
        if parts.len() == 2 {
            headers.insert(
                parts[0].trim().to_string().to_lowercase(),
                parts[1].trim().to_string(),
            );
        }
    }

    let mut content = None;
    if vec!["POST", "PUT"].contains(&method.as_str()) {
        if let Some(size) = headers.get("content-length") {
            let size: usize = size.parse().map_err(|_| std::io::ErrorKind::InvalidInput)?;

            let mut data_buf: Vec<u8> = vec![0; size];
            reader.read(&mut data_buf)?;
            content = Some(String::from_utf8_lossy(&data_buf).to_string());
            buf.append(&mut data_buf);
        }
    }

    Ok(HttpStruct {
        method,
        path,
        version,
        headers,
        content,
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = App::new("http-collector")
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
        )
        .get_matches();

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
    });

    let addr: SocketAddr = SocketAddr::new(IpAddr::from_str(ip)?, port);

    println!("Listening on {:?}", addr);

    let listener = TcpListener::bind(addr)?;

    for connection in listener.incoming() {
        let connection = connection?;
        let config = Arc::clone(&config);
        thread::spawn(move || handle_client(&connection, &config));
    }

    Ok(())
}
