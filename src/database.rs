use futures::{self, Future, Sink, Stream};
use futures::future;
use futures::sync::{mpsc, oneshot};

use error::*;

use mysql_async as mysql;
use mysql_async::{OptsBuilder, Pool};
use mysql_async::prelude::*;

use std::sync::Arc;
use std::thread;

use tokio_core::reactor::{Core, Handle};


type BoxFuture<T> = Box<Future<Item = T, Error = Error>>;


pub struct Client {
    handle: Handle,
    db_loop: mpsc::Sender<Msg>,
    join: thread::JoinHandle<()>,
}

enum Msg {
    FetchChoices(Arc<String>, oneshot::Sender<Vec<(String, String)>>),
}


impl Client {
    pub fn new(db_url: &str, handle: &Handle) -> Client {
        let (db_loop, rx) = mpsc::channel(4);
        let db_url = db_url.to_owned();

        let join = thread::spawn(move || runner(db_url, rx));

        Client {
            db_loop,
            join,
            handle: handle.clone(),
        }
    }

    pub fn fetch_choices(&mut self,
                         query: &Arc<String>)
                         -> futures::BoxFuture<Vec<(String, String)>, Error> {
        let (tx, rx) = oneshot::channel();

        let db_loop = self.db_loop.clone();
        let future = db_loop.send(Msg::FetchChoices(query.clone(), tx));

        self.handle.spawn(future.then(|_| Ok(())));

        rx.map_err(Error::from).boxed()
    }
}

fn runner(db_url: String, rx: mpsc::Receiver<Msg>) {
    let mut core = Core::new().expect("bin::fuzzyd::database: unable to create event loop");
    let handle = core.handle();

    let mut opts = OptsBuilder::from_opts(&db_url);

    opts.pool_min(Some(1_usize));
    opts.pool_max(Some(8_usize));

    let mut pool = Pool::new(opts, &handle);
    debug!("runner: created db pool");

    let msg_loop = rx.and_then(|msg| match msg {
                                   Msg::FetchChoices(query, tx) => {
                                       debug!("runner: executing query: '{}'", query);
                                       fetch_choices(&mut pool, query)
                                           .map(|rows| {
                                                    if let Err(_) = tx.send(rows) {
                debug!("runner: unable to send result, channel closed")
            }
                                                    debug!("runner: fetch choices done");
                                                    ()
                                                })
                                           .or_else(|e| {
            error!("runner: unable to fetch choices: {}", e);
            Ok(())
        })
                                   }
                               });

    core.run(msg_loop.for_each(|_| Ok(()))).unwrap();
}

fn fetch_choices(pool: &mut Pool, query: Arc<String>) -> BoxFuture<Vec<(String, String)>> {
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
