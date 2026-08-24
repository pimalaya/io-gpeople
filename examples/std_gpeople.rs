//! Full std client: list the authenticated user's connections.
//!
//! Opens the TCP and TLS connection to the People API, lists the
//! contacts (connections) with their names, and prints each one. Needs
//! an OAuth 2.0 access token with the contacts scope, read from the
//! `PEOPLE_ACCESS_TOKEN` environment variable:
//!
//! ```sh
//! PEOPLE_ACCESS_TOKEN="<token>" cargo run --example std_people
//! ```

use std::env;

use io_gpeople::v1::{client::PeopleClientStd, rest::people::PeoplePersonField};

fn main() {
    env_logger::try_init().ok();

    let token = env::var("PEOPLE_ACCESS_TOKEN").expect("PEOPLE_ACCESS_TOKEN not set");

    let mut client = PeopleClientStd::connect(token, Default::default()).unwrap();

    let out = client
        .connections_list(&[PeoplePersonField::Names], &Default::default())
        .unwrap();

    for connection in &out.response.connections {
        for name in &connection.names {
            println!("{}: {:?}", connection.resource_name, name.display_name);
        }
    }
}
