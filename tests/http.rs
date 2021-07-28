use std::fs::File;
use std::io::prelude::*;
use std::io::{BufReader, SeekFrom};

use http_collector::http::{parse_http, HttpStruct};

#[test]
fn parse_valid_http_post() -> std::io::Result<()> {
    let f = File::open("test-cases/valid-http-post")?;
    let mut r = BufReader::new(f);

    let mut buf = Vec::new();
    let http = parse_http(&mut r, &mut buf)?;

    assert_eq!(
        http,
        HttpStruct {
            method: "POST".to_string(),
            path: "/".to_string(),
            version: 1.1,
            content: Some("test=1".to_string()),
            headers: vec![
                ("host", "127.0.0.1:8000"),
                ("user-agent", "curl/7.78.0"),
                ("accept", "*/*"),
                ("content-length", "6"),
                ("content-type", "application/x-www-form-urlencoded"),
            ]
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
        }
    );

    let mut buf_test = Vec::new();
    r.seek(SeekFrom::Start(0))?;
    r.read_to_end(&mut buf_test)?;

    assert_eq!(buf, buf_test);

    Ok(())
}

#[test]
fn parse_invalid_http_post() -> std::io::Result<()> {
    let f = File::open("test-cases/invalid-http-post")?;
    let mut r = BufReader::new(f);

    let mut buf = Vec::new();
    let http = parse_http(&mut r, &mut buf);

    match http {
        Ok(_) => assert!(false, "parse_http should return invalid input"),
        Err(e) => assert_eq!(e.kind(), std::io::ErrorKind::InvalidInput),
    };

    Ok(())
}
