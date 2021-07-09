use std::collections::HashMap;

lazy_static! {
    static ref METHODS: HashMap<&'static str, Method> = {
        let mut m = HashMap::new();
        m.insert("GET", Method::GET);
        m.insert("HEAD", Method::HEAD);
        m.insert("POST", Method::POST);
        m.insert("PUT", Method::PUT);
        m.insert("DELETE", Method::DELETE);
        m.insert("CONNECT", Method::CONNECT);
        m.insert("OPTIONS", Method::OPTIONS);
        m.insert("TRACE", Method::TRACE);
        m
    };
}

/*
* RFC 7230, Page 19
*/
#[derive(Debug)]
pub struct HTTPMessage {
    pub request_line: RequestLine,
    pub header_fields: Vec<HeaderField>,
}

/*
* RFC 7230, Page 23
*/
#[derive(Debug)]
pub struct HeaderField {
    pub name: FieldName,
    pub value: FieldValue,
}

/*
* RFC 7230, Page 23
*/
#[derive(Debug)]
pub struct FieldName {
    pub lexeme: String,
}

/*
* RFC 7230, Page 23
*/
#[derive(Debug)]
pub struct FieldValue {
    pub content: Vec<u8>,
}

/*
* RFC 7230, Page 23
*/
#[derive(Debug)]
pub struct FieldContent {
    pub first_char: u8,
    pub second_char: Option<u8>,
}

/*
* RFC 7230, Page 21
*/
#[derive(Debug)]
pub struct RequestLine {
    pub method: Method,
    pub request_target: OriginForm,
    pub http_version: HTTPVersion,
}

/*
* RFC 7231, Page 22
*/
#[derive(Debug, Clone)]
pub enum Method {
    GET,
    HEAD,
    POST,
    PUT,
    DELETE,
    CONNECT,
    OPTIONS,
    TRACE,
}

impl Method {
    pub fn from_string(string: &str) -> Option<Method> {
        METHODS.get(string).cloned()
    }
}

/*
* RFC 7230, Page 41
*/
#[derive(Debug)]
pub struct OriginForm {
    pub absolute_path: AbsolutePath,
    pub query: Option<Query>,
}

/*
* RFC 7230, Page 14
*/
#[derive(Debug)]
pub struct HTTPVersion {
    pub major: u32,
    pub minor: u32,
}

#[derive(Debug)]
pub struct AbsolutePath {
    pub segments: Vec<Segment>,
}

#[derive(Debug)]
pub struct Query {
    pub lexeme: String,
}

#[derive(Debug)]
pub struct Segment {
    pub lexeme: String,
}
