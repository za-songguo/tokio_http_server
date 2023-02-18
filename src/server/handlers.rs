use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use tokio::{fs, io::AsyncWriteExt, net::TcpStream};

use crate::{
    http::{
        request::Request,
        response::{ContentType, HttpStatus, Response},
    },
    SharedData,
};

pub struct Index;
pub struct NotFound;
pub struct VisitCount;
pub struct Echo<'a> {
    pub path_buf: &'a [u8],
}
pub struct StaticFile<'a> {
    // 似乎只能把这个buffer放在这里，如果在handler里面处理就会无法读取请求，导致阻塞线程
    pub path_buf: &'a [u8],
}

#[async_trait]
pub trait Handler {
    async fn handle(&self, stream: &mut TcpStream, shared_data: Arc<Mutex<SharedData>>);
}

#[async_trait]
impl Handler for Index {
    async fn handle(&self, stream: &mut TcpStream, _shared_data: Arc<Mutex<SharedData>>) {
        let body = "Index Page";

        let mut response = Response::new();
        let response = response
            .set_status(HttpStatus::Ok)
            .set_headers("Content-Type".into(), ContentType::Html.to_string())
            .set_headers("Content-Length".into(), body.len().to_string())
            .set_body(body.as_bytes());

        stream.write_all(&response.as_bytes()).await.unwrap();
        stream.flush().await.unwrap();
    }
}

#[async_trait]
impl Handler for VisitCount {
    async fn handle(&self, stream: &mut TcpStream, shared_data: Arc<Mutex<SharedData>>) {
        shared_data.lock().unwrap().visit_count += 1;
        let visit_count = shared_data.lock().unwrap().visit_count;

        let body = format!("{} Times!", visit_count);

        let mut response = Response::new();
        let response = response
            .set_status(HttpStatus::Ok)
            .set_headers("Content-Type".into(), ContentType::Html.to_string())
            .set_headers("Content-Length".into(), body.len().to_string())
            .set_body(body.as_bytes());

        stream.write_all(&response.as_bytes()).await.unwrap();
        stream.flush().await.unwrap();
    }
}

#[async_trait]
impl Handler for Echo<'_> {
    async fn handle(&self, stream: &mut TcpStream, _shared_data: Arc<Mutex<SharedData>>) {
        let req: Request = self.path_buf.to_vec().into();
        let queries = req.parse_queries();

        let body = queries.get("content").unwrap_or(&"Need some arguments");

        let mut response = Response::new();
        let response = response
            .set_status(HttpStatus::Ok)
            .set_headers("Content-Type".into(), ContentType::Html.to_string())
            .set_headers("Content-Length".into(), body.len().to_string())
            .set_body(body.as_bytes());

        stream.write_all(&response.as_bytes()).await.unwrap();
        stream.flush().await.unwrap();
    }
}

#[async_trait]
impl Handler for StaticFile<'_> {
    async fn handle(&self, stream: &mut TcpStream, shared_data: Arc<Mutex<SharedData>>) {
        // Static file path
        // Read the path from request (struct)
        let buf = &String::from_utf8_lossy(self.path_buf); // Buffer to string

        // GET /static/...
        for i in buf.split_whitespace() {
            if i.contains("/static/") {
                let path = i.split('/').collect::<Vec<&str>>()[2]; // static/<path>
                let file = fs::read(format!("static/{path}")).await;

                // 也可以直接使用URL里的 `static/<path>`，不过像上面这样写可以用在当静态文件的文件夹不是 `static` 的时候
                // let path = i.strip_prefix('/').unwrap_or_default();
                // let file = fs::read(path).await;

                if let Ok(f) = file {
                    let content_type = parse_content_type(path);

                    let mut response = Response::new();
                    let response = response
                        .set_status(HttpStatus::Ok)
                        .set_headers("Content-Type".into(), content_type.to_string())
                        .set_headers("Content-Length".into(), f.len().to_string())
                        .set_body(&f);

                    stream.write_all(&response.as_bytes()).await.unwrap();
                    stream.flush().await.unwrap();
                } else {
                    NotFound.handle(stream, Arc::clone(&shared_data)).await;
                }
            }
        }
    }
}

#[async_trait]
impl Handler for NotFound {
    async fn handle(&self, stream: &mut TcpStream, _shared_data: Arc<Mutex<SharedData>>) {
        let body = "404 Not Found!";

        let mut response = Response::new();
        let response = response
            .set_status(HttpStatus::NotFound)
            .set_headers("Content-Type".into(), ContentType::Html.to_string())
            .set_headers("Content-Length".into(), body.len().to_string())
            .set_body(body.as_bytes());

        stream.write_all(&response.as_bytes()).await.unwrap();
        stream.flush().await.unwrap();
    }
}
// Parse the `Content-Type` from request
fn parse_content_type(req: &str) -> ContentType {
    // .html or .htm
    if req.contains(".htm") {
        ContentType::Html
    } else if req.contains(".txt") {
        ContentType::PlainText
    } else if req.contains(".css") {
        ContentType::Css
    } else if req.contains(".png") || req.contains(".jpg") || req.contains(".ico") {
        ContentType::AvifImage
    } else {
        ContentType::Html
    }
}
