This is a short, self-contained, correct example [SSCCE](http://sscce.org) for
debugging purposes from a much larger project.

# The Problem

This program runs a simple database `SELECT` query intended to pull a large
quantity of results (100k+).  This issue I see is that the program will
randomly fail with a `Packet out of order` error.


# Expected Behavior

If the query can run once successfully, I expect it to run successfully every
time, barring any external factors (hardware failures, unreliable network, etc).


# Steps to Reproduce

1.  Grab your favorite MySQL derivative and spin up a database server.  I've
    tested against MySQL 5.5 and MariaDB 10.1.21.
2.  Add a test user to your database.  If you prefer more security, specify
    your host IP instead of the host wildcard `'%'`.

    ```sql
    CREATE USER 'testbot'@'%' IDENTIFIED BY 'testbot';
    GRANT ALL PRIVILEGES ON `fake_data`.* TO 'testbot'@'%';
    ```

3.  Add a database for testing and put some fake data into it...

    1.  Edit the global variables in `tools/fake-data-gen.py` script.  Change
        `db_host` and `db_port` to reflect the database settings for your
        environment.
    2.  Install `python 2.7` and `pip`.
    3.  Install two dependencies for the fake data script:

        *  `pip install faker`

        *  `pip install mysql-connector==2.1.4`.  The latest version of this
            package would not install for me.

    4.  Run `python2 ./tools/fake-data-gen.py`.  If no errors are reported, you
        should see a `fake_data` database on your MySQL server with a `products`
        table full of random data.

4.  Run the binary.

    ```sh
    export DSN="mysql://testbot:testbot@host:port/fake_data"
    RUST_LOG=bug_test_01=debug,mysql_async=debug cargo run --release --bin bug-test-01
    ```

5.  Observe that the programs fails with an error similar to:

        DEBUG:mysql_async::proto: Last seq id 211
        DEBUG:mysql_async::proto: Last seq id 48
        ERROR:bug_test_01::database: runner: unable to fetch choices: Packet out of order
        ERROR:bug_test_01: oneshot canceled
        DEBUG:bug_test_01: done

    If you do not see this error, rerun the binary until you do.  Like I said,
    the error occurs randomly.  To improve your chances of the error occurring,
    remove the `RUST_LOG` environment variable to disable debugging print
    statements, or only remove `mysql_async=debug` from that log list.


# Things I've Tried or...

## How I know this isn't a network/database server issue.

On the very same machine I run `cargo run --release --bin bug-test-01`, I fired
up [Wireshark][wireshark] and captured the network packets to and from the
database server on port 3306.

[wireshark]: https://www.wireshark.org/

I then analyzed the [SQL protocol][sql-protocol] in the packet capture and
found no sequence numbers out of order, and other than the program closing the
TCP connection early with RST packets after the `Packet out of order` error,
everything appears to transferring smoothly from the database server to my
computer's network interface card (NIC).  *NOTE*, if you choose to pursue this
exercise yourself, the MySQL dissector is invaluable, but buggy; you may run
into "Malformed MySQL Packets" in Wireshark.  I recommend compiling Wireshark
from scratch with the patch mentioned in [this bug
report][wireshark-bug-report] to disable SQL protocol compression detection.

[sql-protocol]: https://mariadb.com/kb/en/mariadb/clientserver-protocol/
[wireshark-bug-report]: https://bugs.wireshark.org/bugzilla/show_bug.cgi?id=13754

I also used the command below (`tshark` is a commandline utility provided with
Wireshark) to print all the MySQL packet sequence numbers captured.  My
captures showed all sequence numbers in order.

```sh
./tshark -r PATH-TO-CAPTURE-FILE -T fields -e mysql.packet_number | egrep -v '^$' | sed 's/,/\n/g' | less
```


## Why I can't add more debug print statements to find the problem.

I did this, and the problem stopped manifesting.  I forked
[`mysql_async`][mysql-async] and added a `debug!()` statement in
`src/conn/futures/read_packet.rs:43`--it prints with every attempt to
process/validate each MySQL packet--and in `src/proto.rs:304`--this prints with
every attempt to parse each MySQL packet.

Apparently, giving the event loop (that decodes the packets) the extra bit of
work to print to `stderr` delays the program just enough that it doesn't fail
(so long as I have `RUST_LOG=mysql_async=debug` enabled.

[mysql-async]: https://github.com/blackbeam/mysql_async/

This indicates to me that I have a race condition of sorts in my program.  Not
necessarily in my code, but I rather suspect it's in the libraries I'm using.
Either `mysql_async` itself, or the `tokio-rs`/`futures`/`mio` framework.
While this example spins up a second thread, all the database work is done in
one thread (specifically the thread created by `tokio_core::reactor::Core`.  My
second thread just waits for a result.

Besides, isn't Rust supposed to prevent race conditions from occurring?  At
least so far as my code is concerned, it's 100% Rust with no `unsafe` blocks.

Heck, while debugging a separate issue with this project, I introduced myself
to the `perftools` on Linux and `perf report` showed that my program spent over
50% of its time zeroing the 4kB local stack buffer used by
[`mysql_async::io::Stream`][io-mod].  So I'm not even sure where the garbage
value corrupting the MySQL packet sequence number could be coming from, if not
the `tokio-rs` libraries further down the software stack, (e.g.
`tokio::net::TcpStream`).

[io-mod]: https://github.com/blackbeam/mysql_async/blob/v0.9.3/src/io/mod.rs#L102


# My Appeal to You

Where do I go from here to isolate and trap this bug?

I don't use `gdb` because I'm a novice at best with that program.  I can print
variables, step, continue, and show backtraces, but I'm not productive with the
debugger.  If I use it, I'm stepping through every line in the program and
that's a huge time sink, not to mention that slowing down the rate at which the
binary processes network traffic prevents the problem from occurring.

I'm not comfortable side-stepping the issue.  Teaching my code to retry a fixed
number of times and give up is an option, but it complicates my code and
reduces its utility.  If I can identify the cause and fix it, I'd rather do
that.  I've pretty much exhausted all the tricks in my toolbox, and I am
stalled, so here's hoping someone out there has options in their toolbox to
share.

I've also tried reading the source code for `mysql_async`, the various
`tokior-rs` libraries I use, and it's a bit beyond my capacity to read that
code and infer the why's and contexts and invariant assumptions that
established such code.  The very little I've gleaned from the `tokio-rs` guides
is that `tokio-rs` has a layered approach to I/O, so if the protocol handler
isn't given enough buffered data to decode the next packet/message, then its
return value is used to indicate that the buffer wasn't used and needs more
data.



Copyright (c) 2017 Justin Charette (boxofrox)
