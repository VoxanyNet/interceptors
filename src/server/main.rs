use crate::server::Server;

pub mod server;

pub fn main() {

    let mut server = Server::new();

    server.run();
}