use crate::server::Server;

pub mod server;

pub fn main() {
    pretty_env_logger::init();

    let mut server = Server::new();

    server.run();
}