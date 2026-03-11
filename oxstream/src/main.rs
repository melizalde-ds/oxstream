use anyhow::{Result, anyhow};
use gst::Pipeline;
use tracing::info;

fn main() -> Result<()> {
    init_tracing();
    init_gst()?;
    info!("GStreamer initialized successfully");
    let _pipeline = Pipeline::new();
    info!("Pipeline created successfully");
    Ok(())
}

fn init_tracing() {
    tracing_subscriber::fmt::init();
}

fn init_gst() -> Result<()> {
    if let Err(err) = gst::init() {
        tracing::error!("Failed to initialize GStreamer: {}", err);
        return Err(anyhow!("Failed to initialize GStreamer"));
    }
    Ok(())
}
