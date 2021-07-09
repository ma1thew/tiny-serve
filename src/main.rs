#[macro_use]
extern crate lazy_static;

mod http;
mod peekable_bufreader;
mod response;
mod chunked_bufreader;

use std::env;
use std::path::PathBuf;
use std::path::Path;

use futures::stream;
use futures::stream::StreamExt;

use async_std::prelude::*;
use async_std::task;
use async_std::net::{TcpStream, TcpListener};
use async_std::io::{BufReader, BufWriter};
use async_std::fs::File;
use async_std::fs;

use chrono::offset::Local;
use chrono::DateTime;

use http::{Parser, Method};
use peekable_bufreader::PeekableBufReader;
use chunked_bufreader::ChunkedBufReader;
use response::{Response, BadRequest, NotFound, NotImplemented, InternalServerError};

#[async_std::main]
async fn main() {
    let port = env::args().skip(1).next().unwrap_or("8000".to_owned()).parse::<u16>().unwrap_or(8000);
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await;
    match listener {
        Err(e) => {
            eprintln!("Failed to bind TCP Listener: {}", e);
            return;
        },
        _ => {},
    }
    let listener = listener.unwrap();
    listener
        .incoming()
        .for_each_concurrent(None, |stream| async move {
            if let Ok(valid) = stream {
                task::spawn(handle_connection(valid));
            }
        })
        .await;
}

async fn handle_connection(mut stream: TcpStream) {
    let response = generate_response(&stream).await.response_bytes();
    let writer = BufWriter::new(&mut stream);
    let mut writer = response.fold(writer, |mut writer, bytes| async move {
        writer.write(&bytes).await.unwrap();
        writer
    }).await;
    writer.flush().await.unwrap();
    stream.flush().await.unwrap();
}

async fn generate_response(stream: &TcpStream) -> Box<dyn Response> {
    let reader = PeekableBufReader::new(BufReader::new(stream));
    let request = match Parser::new(reader).parse().await {
        Some(success) => success,
        None => return Box::new(BadRequest{}),
    };
    match request.method {
        Method::GET => {
            if request.requested_path.iter().filter(|segment| segment.contains("/")).count() != 0 {
                return Box::new(BadRequest{});
            } else {
                let path = match PathBuf::from("./".to_owned() + &request.requested_path.join("/")).canonicalize() {
                    Ok(canonical_path) => canonical_path,
                    Err(_) => return Box::new(NotFound{}),
                };
                let current_dir = match env::current_dir() {
                    Ok(dir) => match dir.canonicalize() {
                        Ok(canonical_path) => canonical_path,
                        Err(_) => return Box::new(InternalServerError{}),
                    },
                    Err(_) => return Box::new(InternalServerError{}),
                };
                if !is_path_ancestor_of(&current_dir, &path) {
                    return Box::new(NotFound{});
                }
                let metadata = match fs::metadata(&path).await {
                    Ok(data) => data,
                    Err(_) => return Box::new(NotFound{}),
                };
                if metadata.is_dir() {
                    let friendly_name = match path.strip_prefix(&current_dir) {
                        Ok(result) => {
                            match result.to_str() {
                                Some(result) => result,
                                None => return Box::new(InternalServerError{}),
                            }
                        },
                        Err(_) => return Box::new(InternalServerError{}),
                    };
                    let mut listings = Vec::new();
                    if path != current_dir {
                        listings.push(format!(include_str!("../res/listing_entry.html"), path.parent().unwrap().strip_prefix(&current_dir).unwrap().to_str().unwrap(), "..", "-", "-"));
                    }
                    for file in path.read_dir().unwrap() {
                        let file_path = file.unwrap().path();
                        let file_metadata = fs::metadata(&file_path).await.unwrap();
                        let created_time = match file_metadata.created() {
                            Ok(birth_time) => {
                                let formatted_time: DateTime<Local> = birth_time.into();
                                formatted_time.format("%d-%b-%Y %H:%M").to_string()
                            },
                            Err(_) => "-".to_owned(),
                        };
                        let file_size = if file_metadata.is_dir() {
                            "-".to_owned()
                        } else {
                            file_metadata.len().to_string()
                        };
                        listings.push(format!(include_str!("../res/listing_entry.html"),
                            file_path.strip_prefix(&current_dir).unwrap().to_str().unwrap(),
                            file_path.file_name().unwrap().to_str().unwrap(),
                            created_time,
                            file_size,
                        ))
                    }
                    return Box::new(response::Ok {
                        file_stream: Box::new(stream::iter(vec![Vec::from(
                                format!(
                                    include_str!("../res/listing.html"),
                                    friendly_name,
                                    friendly_name,
                                    listings.join("\n"),
                                ).as_bytes()
                            )].into_iter().map(|entry| entry.to_owned())))
                    });
                } else {
                    return Box::new(response::Ok{ file_stream: Box::new(ChunkedBufReader::new(BufReader::new(File::open(&path).await.unwrap()))) })
                }
            }
        },
        _ => return Box::new(NotImplemented{}),
    }
}

fn is_path_ancestor_of(ancestor: &Path, child: &Path) -> bool {
    let mut ancestors = child.ancestors();
    loop {
        if let Some(parent) = ancestors.next() {
            if parent == ancestor {
                return true;
            }
        } else {
            return false;
        }
    }
}
