use std::collections::HashMap;

use super::rule::{HTTPMessage, Method};

#[derive(Debug)]
pub enum Version {
    Http0_9,
    Http1_0,
    Http1_1,
    // HTTP/2 won't parse, anyway.
}

#[derive(Debug)]
pub struct HTTPRequest {
    pub method: Method,
    pub version: Version,
    pub requested_path: Vec<String>,
    pub headers: HashMap<String, Vec<u8>>,
}

pub fn flatten(message: HTTPMessage) -> Option<HTTPRequest> {
    let method = message.request_line.method;
    let version = match (message.request_line.http_version.major, message.request_line.http_version.minor) {
        (0, 9) => Version::Http0_9,
        (1, 0) => Version::Http1_0,
        (1, 1) => Version::Http1_1,
        _ => return None,
    };
    let requested_path = message.request_line.request_target.absolute_path.segments.into_iter().map(|segment| segment.lexeme).collect();
    let headers = message.header_fields.into_iter().map(|field| (field.name.lexeme, field.value.content)).collect();
    Some(HTTPRequest{
        method,
        version,
        requested_path,
        headers,
    })
}
