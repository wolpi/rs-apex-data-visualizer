mod parse;

#[macro_use]
extern crate actix_web;

extern crate serde_json;

extern crate url;

use actix_http::{body::Body, Response};
use actix_web::{App, HttpRequest, HttpResponse, HttpServer, Result};
use actix_web::dev::ServiceResponse;
use actix_web::http::StatusCode;
use actix_web::middleware;
use actix_web::middleware::errhandlers::{ErrorHandlerResponse, ErrorHandlers};
use actix_web_static_files;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::Mutex;
use std::fs::File;
use std::path::Path;
use std::io;
use std::env;
use std::ops::Deref;

lazy_static! {
    static ref DATA: Mutex<HashMap<String, Vec<parse::Entry>>> = Mutex::new(HashMap::new());
}

#[get("/data.json")]
async fn data_json(req: HttpRequest) -> HttpResponse {
    println!("data.json request from {}", req.connection_info().remote_addr().unwrap());
    HttpResponse::Ok().json(&DATA.lock().unwrap().deref())
}

#[get("/data-apex.json")]
async fn data_apex_json(req: HttpRequest) -> HttpResponse {
    println!("data-apex.json request from {}", req.connection_info().remote_addr().unwrap());
    let mut map_apex :HashMap<&str, Vec<(&u64, &f32)>> = HashMap::new();
    let data = DATA.lock().unwrap();
    let map = data.deref();
    for (name, vec) in map.iter() {
        let vec_apex :Vec<(&u64, &f32)> = vec.into_iter().map(
            |entry| (&entry.timestamp, &entry.value)
        ).collect();
        map_apex.insert(name, vec_apex);
    }
    HttpResponse::Ok().json(map_apex)
}

fn error_handlers() -> ErrorHandlers<Body> {
    ErrorHandlers::new().handler(StatusCode::NOT_FOUND, not_found)
}

fn not_found<B>(res: ServiceResponse<B>) -> Result<ErrorHandlerResponse<B>> {
    let response = get_error_response(&res, "Page not found");
    Ok(ErrorHandlerResponse::Response(
        res.into_response(response.into_body()),
    ))
}

fn get_error_response<B>(res: &ServiceResponse<B>, error: &'static str) -> Response<Body> {
    Response::build(res.status())
        .content_type("text/plain")
        .body(error)
}

fn handle_file(path_str :&String) -> io::Result<&'static str> {
    let data_path = Path::new(path_str);
    let file_result = File::open(&data_path);
    if file_result.is_err() {
        println!("could not open file: '{}'!!!", data_path.to_str().unwrap());
        let err = file_result.unwrap_err();
        println!("{}", err);
        return io::Result::Err(err);
    }
    let file_name = data_path.file_name().unwrap().to_str().unwrap();
    let fallback_name = extract_fallback_name(file_name);
    println!("parsing file with fallback name: {}, {}", fallback_name, path_str);
    init_data(&file_result.unwrap(), fallback_name);
    return Result::Ok("ok");
}

fn extract_fallback_name(file_name :&str) -> &str {
    let index_opt_dot = file_name.find(".");
    let index_opt_under = file_name.find("_");

    let index_dot = if index_opt_dot.is_some() {index_opt_dot.unwrap()} else {file_name.len()};
    let index_under = if index_opt_under.is_some() {index_opt_under.unwrap()} else {file_name.len()};

    let index = std::cmp::min(index_dot, index_under);
    return &file_name[0 .. index];
}

fn init_data(file: &File, fallback_name :&str) {
    let mut data: HashMap<String, Vec<parse::Entry>> = HashMap::new();
    parse::parse_file(&file, fallback_name, &mut data);
    let mut guard = DATA.lock().unwrap();
    for (name, vec) in data.iter_mut() {
        vec.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        let mut copy :Vec<parse::Entry> = Vec::new();
        for entry in vec {
            copy.push(entry.clone());
        }
        guard.insert(name.to_string(), copy);
    }
}

include!(concat!(env!("OUT_DIR"), "/generated.rs"));

#[actix_web::main]
async fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().enumerate().filter(|(index, _value)| *index > 0 as usize).map(|(_index, value)| value).collect();
    if args.len() < 1 {
        return io::Result::Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "which data file to serve ???"));
    }
    let mut port :&str = "8080";
    let mut skip_next = false;
    for i in 0..args.len() {
        if skip_next {
            skip_next = false;
            continue;
        }
        let arg = &args[i];
        if arg == "-p" {
            if args.len() > (i + 1) {
                port = &args[i+1];
                skip_next = true;
            }
        } else {
            let file_result = handle_file(&arg);
            if file_result.is_err() {
                let error_msg = format!("could not open file {}: {}", arg, file_result.err().unwrap());
                return io::Result::Err(io::Error::new(io::ErrorKind::InvalidInput, error_msg));
            }
        }
    }

    let bind = format!("0.0.0.0:{}", port);
    println!("binding to {}", bind);

    HttpServer::new(move || {
        let generated = generate();
        App::new()
            .wrap(error_handlers())
            .wrap(middleware::DefaultHeaders::new().header("Cache-Control", "max-age=0"))
            // register data_json before static files on /
            .service(data_json)
            .service(data_apex_json)
            .service(actix_web_static_files::ResourceFiles::new("/", generated,))
    })
    .bind(bind)?
    .run()
    .await
}
