use anyhow::{Result, anyhow};
use gst::{Element, ElementFactory, Pipeline};
use tracing::{error, info};

fn main() -> Result<()> {
    init_tracing();
    init_gst()?;
    info!("GStreamer initialized successfully");
    let _pipeline = Pipeline::new();
    info!("Pipeline created successfully");
    let _src = make_src("pipewiresrc")?;
    info!("Source element created successfully");
    Ok(())
}

fn init_tracing() {
    tracing_subscriber::fmt::init();
}

fn init_gst() -> Result<()> {
    if let Err(err) = gst::init() {
        error!("Failed to initialize GStreamer: {}", err);
        return Err(anyhow!("Failed to initialize GStreamer"));
    }
    Ok(())
}

fn make_src(title: &str) -> Result<Element> {
    let _ = match ElementFactory::find(title) {
        Some(factory) => factory,
        None => {
            error!("Element factory for '{}' not found", title);
            return Err(anyhow!("Element factory for '{}' not found", title));
        }
    };
    match ElementFactory::make(title).build() {
        Ok(element) => Ok(element),
        Err(err) => {
            error!("Failed to create element 'pipewiresrc': {}", err);
            Err(anyhow!("Failed to create element 'pipewiresrc': {}", err))
        }
    }
}
