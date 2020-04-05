use chrono;
use std::io;

error_chain! {
    foreign_links {
        ChronoParse(chrono::format::ParseError);
        IO(io::Error);
    }
}
