#[macro_use]
extern crate error_chain;
extern crate env_logger;
extern crate futures;
#[macro_use]
extern crate log;
extern crate mysql_async;
extern crate tokio_core;

mod database;
mod error;

use database::*;
use error::*;

use futures::Future;

use std::env;
use std::sync::Arc;

use tokio_core::reactor::Core;


fn main() {
    env_logger::init()
        .chain_err(|| "cannot initialize logger")
        .unwrap();

    debug!("running");

    let db_url = env::var("DSN").unwrap();

    let mut core = Core::new().unwrap();

    let mut db = Client::new(&db_url, &core.handle());

    let query = Arc::new("SELECT sku, description FROM products".to_owned());

    let future = db.fetch_choices(&query)
        .then(|result| {
                  if let Err(err) = result {
                      error!("{}", err);
                  }
                  Ok(()) as Result<()>
              })
        .boxed();

    core.run(future)
        .chain_err(|| "error resolving future")
        .expect("error resolving future");

    debug!("done");
}
