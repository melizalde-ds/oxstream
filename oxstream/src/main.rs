use tracing::info;

fn main() {
    init_tracing();
    info!("Hello, world!");
}

fn init_tracing() {
    tracing_subscriber::fmt::init();
}
