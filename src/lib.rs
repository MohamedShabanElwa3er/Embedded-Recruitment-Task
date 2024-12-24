pub mod server;

pub mod message {
    include!(concat!(env!("OUT_DIR"), "/messages.rs"));
}

use std::io;

/// Utility function to initialize the logger
pub fn initialize_logger() -> io::Result<()> {
    use env_logger::Builder;
    use std::env;
    Builder::new()
        .parse_filters(&env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()))
        .init();
    Ok(())
}
