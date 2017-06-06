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

use mysql_async::{OptsBuilder, Pool};

use std::env;
use std::sync::Arc;

use tokio_core::reactor::Core;


fn main() {
    env_logger::init()
        .chain_err(|| "cannot initialize logger")
        .unwrap();

    debug!("running");

    let db_url = env::var("DSN").unwrap();
    let opts = OptsBuilder::from_opts(&db_url);

    let mut core = Core::new().unwrap();
    let mut pool = Pool::new(opts, &core.handle());

    let query = Arc::new("SELECT sku, description FROM products".to_owned());

    let future = fetch_choices(&mut pool, query)
        .then(|result| {
                  if let Err(err) = result {
                      error!("{}", err);
                  }
                  Ok(()) as Result<()>
              });

    core.run(future)
        .chain_err(|| "error resolving future")
        .expect("error resolving future");

    debug!("done");
}
