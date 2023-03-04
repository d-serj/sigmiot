
use std::sync::{Mutex, Condvar, Arc};

use anyhow::Error;
use embedded_svc::http::server::{
    Connection, Handler, HandlerResult, Method, Middleware, Query, Request, Response,
};
use embedded_svc::io::Write;
use esp_idf_svc::http::server::{fn_handler, EspHttpConnection, EspHttpServer};

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
    mutex: Arc<(Mutex<Option<u32>>, Condvar)>,
) -> Result<esp_idf_svc::http::server::EspHttpServer, Error> {

    struct SampleMiddleware {}

    impl<C> Middleware<C> for SampleMiddleware
    where
        C: Connection,
    {
        fn handle<'a, H>(&'a self, connection: &'a mut C, handler: &'a H) -> HandlerResult
        where
            H: Handler<C>,
        {
            let req = Request::wrap(connection);

            println!("Middleware called with uri: {}", req.uri());

            let connection = req.release();

            if let Err(err) = handler.handle(connection) {
                if !connection.is_response_initiated() {
                    let mut resp = Request::wrap(connection).into_status_response(500)?;

                    write!(&mut resp, "ERROR: {err}")?;
                } else {
                    // Nothing can be done as the error happened after the response was initiated, propagate further
                    return Err(err);
                }
            }

            Ok(())
        }
    }

    struct SampleMiddleware2 {}

    impl<C> Middleware<C> for SampleMiddleware2
    where
        C: Connection,
    {
        fn handle<'a, H>(&'a self, connection: &'a mut C, handler: &'a H) -> HandlerResult
        where
            H: Handler<C>,
        {
            println!("Middleware2 called");

            handler.handle(connection)
        }
    }


    let mut server =  EspHttpServer::new(&Default::default())?;

    server
    .fn_handler("/", Method::Get, |req| {
        req.into_ok_response()?
            .write_all("Hello from Rust!".as_bytes())?;

        Ok(())
    })?
    .fn_handler("/foo", Method::Get, |_| {
        Result::Err("Boo, something happened!".into())
    })?
    .fn_handler("/bar", Method::Get, |req| {
        req.into_response(403, Some("No permissions"), &[])?
            .write_all("You have no permissions to access this page".as_bytes())?;

        Ok(())
    })?
    .fn_handler("/panic", Method::Get, |_| panic!("User requested a panic!"))?
    .handler(
        "/middleware",
        Method::Get,
        SampleMiddleware {}.compose(fn_handler(|_| {
            Result::Err("Boo, something happened!".into())
        })),
    )?
    .handler(
        "/middleware2",
        Method::Get,
        SampleMiddleware2 {}.compose(SampleMiddleware {}.compose(fn_handler(|req| {
            req.into_ok_response()?
                .write_all("Middleware2 handler called".as_bytes())?;

            Ok(())
        }))),
    )?;

    Ok(server)
}