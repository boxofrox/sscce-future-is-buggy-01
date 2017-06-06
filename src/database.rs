use futures::{future, Future};

use error::*;

use mysql_async as mysql;
use mysql_async::Pool;
use mysql_async::prelude::*;

use std::sync::Arc;


type BoxFuture<T> = Box<Future<Item = T, Error = Error>>;


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
