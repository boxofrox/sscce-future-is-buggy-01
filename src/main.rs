#[macro_use]
extern crate error_chain;
extern crate env_logger;
extern crate futures;
#[macro_use]
extern crate log;
extern crate mysql_async;
extern crate tokio_core;

use futures::Future;

use mysql_async::{OptsBuilder, Pool};
use mysql_async::prelude::*;

use std::env;

use tokio_core::reactor::Core;


fn main() {
    env_logger::init()
        .chain_err(|| "cannot initialize logger")
        .unwrap();

    debug!("running");

    let db_url = env::var("DSN").unwrap();
    let opts = OptsBuilder::from_opts(&db_url);

    let mut core = Core::new().unwrap();
    let pool = Pool::new(opts, &core.handle());

    let fetch_results = pool.get_conn()
        .and_then(|conn| conn.query("SELECT sku, description FROM products"))
        .and_then(|result| result.drop_result())
        .then(|result| {
                  if let Err(err) = result {
                      error!("{}", err);
                  }
                  Ok(()) as Result<()>
              });

    core.run(fetch_results)
        .chain_err(|| "error resolving future")
        .expect("error resolving future");

    debug!("done");
}


error_chain!{
    foreign_links {
        Mysql(::mysql_async::errors::Error);
    }
}
