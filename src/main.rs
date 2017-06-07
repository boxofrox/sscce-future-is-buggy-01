#[macro_use]
extern crate error_chain;
extern crate env_logger;
extern crate futures;
#[macro_use]
extern crate log;
extern crate mysql_async;
extern crate tokio_core;

use futures::{future, Future};

use mysql_async as mysql;
use mysql_async::{OptsBuilder, Pool};
use mysql_async::prelude::*;

use std::env;
use std::sync::Arc;

use tokio_core::reactor::Core;


type BoxFuture<T> = Box<Future<Item = T, Error = Error>>;


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


pub fn fetch_choices(pool: &mut Pool, query: Arc<String>) -> BoxFuture<Vec<(String, String)>> {
    let pool = pool.clone();

    let future = pool.get_conn()
        .and_then(move |conn| conn.query(query.as_ref()))
        .and_then(|result| {
                      result.map(|row| {
                let (id, text): (String, Option<String>) = mysql::from_row(row);
                (id, text.unwrap_or("".to_owned()))
            })
                  })
        .and_then(|(rows, _conn)| future::ok(rows))
        .map_err(Error::from);

    Box::new(future)
}


error_chain!{
    foreign_links {
        Mysql(::mysql_async::errors::Error);
    }
}
