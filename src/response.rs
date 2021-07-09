use futures::prelude::*;

pub trait Response: Send + Sync {
    fn response_bytes(self: Box<Self>) -> Box<dyn Stream<Item = Vec<u8>> + Unpin + Send>;
}

pub struct Ok {
    pub file_stream: Box<dyn Stream<Item = Vec<u8>> + Unpin + Send + Sync>,
}

impl Response for Ok {
    fn response_bytes(self: Box<Self>) -> Box<dyn Stream<Item = Vec<u8>> + Unpin + Send> {
        Box::new(stream::iter(vec![Vec::from(b"HTTP/1.1 200 OK\r\n\r\n" as &[u8])].into_iter().map(|entry| entry.to_owned())).chain(self.file_stream))
    }
}

pub struct BadRequest {

}

impl Response for BadRequest {
    fn response_bytes(self: Box<Self>) -> Box<dyn Stream<Item = Vec<u8>> + Unpin + Send> {
        Box::new(stream::iter(vec![Vec::from(b"HTTP/1.1 400 Bad Request\r\n\r\n" as &[u8]), include_bytes!("../res/400.html").to_vec()].into_iter().map(|entry| entry.to_owned())))
    }
}

pub struct NotFound {

}

impl Response for NotFound {
    fn response_bytes(self: Box<Self>) -> Box<dyn Stream<Item = Vec<u8>> + Unpin + Send> {
        Box::new(stream::iter(vec![Vec::from(b"HTTP/1.1 404 Not Found\r\n\r\n" as &[u8]), include_bytes!("../res/404.html").to_vec()].into_iter().map(|entry| entry.to_owned())))
    }
}

pub struct NotImplemented {

}

impl Response for NotImplemented {
    fn response_bytes(self: Box<Self>) -> Box<dyn Stream<Item = Vec<u8>> + Unpin + Send> {
        Box::new(stream::iter(vec![Vec::from(b"HTTP/1.1 501 Not Implemented\r\n\r\n" as &[u8]), include_bytes!("../res/501.html").to_vec()].into_iter().map(|entry| entry.to_owned())))
    }
}

pub struct InternalServerError {

}

impl Response for InternalServerError {
    fn response_bytes(self: Box<Self>) -> Box<dyn Stream<Item = Vec<u8>> + Unpin + Send> {
        Box::new(stream::iter(vec![Vec::from(b"HTTP/1.1 500 InternalServerError\r\n\r\n" as &[u8]), include_bytes!("../res/500.html").to_vec()].into_iter().map(|entry| entry.to_owned())))
    }
}
