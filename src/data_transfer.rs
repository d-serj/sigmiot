
use std::sync::{Mutex, Condvar, Arc};

use anyhow::Error;
use embedded_svc::http::server::{
    Connection, Handler, HandlerResult, Method, Middleware, Query, Request, Response,
};
use embedded_svc::io::Write;
use esp_idf_svc::http::server::{fn_handler, EspHttpServer};
use crate::data_provider::DataProvider;

pub trait DataTransfer {
    fn init(&mut self) -> Result<(), Error>;
    fn send_data(&self, data: &DataProvider) -> Result<(), Error>;
}

pub struct HttpServer {

}

impl HttpServer {
    fn new() -> Self {
        Self { }
    }

   // fn post()
}

impl DataTransfer for HttpServer {
    fn init(&mut self) -> Result<(), Error> {
        todo!()
    }

    fn send_data(&self, data: &DataProvider) -> Result<(), Error> {
        todo!()
    }
}

pub fn httpd(
    data: Arc<Mutex<DataProvider>>,
) -> Result<esp_idf_svc::http::server::EspHttpServer, Error> {

    let mut server =  EspHttpServer::new(&Default::default())?;

    server
    .fn_handler("/", Method::Get, move|req| {
        let clone = data.clone();
        let raw_data = clone.lock().unwrap().get_http_data();
        req.into_ok_response()?
            .write_all(raw_data.as_bytes())?;

        Ok(())
    })?;

    Ok(server)
}