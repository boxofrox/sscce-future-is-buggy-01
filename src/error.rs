error_chain!{
    foreign_links {
        OneshotCancelled(::futures::sync::oneshot::Canceled);
        Io(::std::io::Error);
        Mysql(::mysql_async::errors::Error);
    }
}
