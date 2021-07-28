use std::collections::HashMap;
use std::fmt;
use std::io::BufReader;
use std::io::{BufRead, Read};

#[cfg(feature = "json")]
use serde::Serialize;

#[cfg_attr(feature = "json", derive(Serialize))]
pub struct HttpStruct {
    pub method: String,
    pub path: String,
    pub version: f64,
    pub headers: HashMap<String, String>,
    pub content: Option<String>,
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

pub fn parse_http<R: std::io::Read>(
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
    let version: f64 = version_parts[1]
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
