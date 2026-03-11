use anyhow::{Result, anyhow};
use gst::prelude::*;
use gst::{
    Element, ElementFactory, Pipeline, glib::object::ObjectExt, prelude::GObjectExtManualGst,
};
use tracing::{error, info};

fn main() -> Result<()> {
    init_tracing();
    init_gst()?;
    info!("GStreamer initialized successfully");

    let src = make_element("pipewiresrc", None)?;
    info!("Source element created successfully");

    let conv = make_element("videoconvert", None)?;
    info!("Video converter element created successfully");

    let sink = make_element("autovideosink", None)?;
    info!("Video sink element created successfully");

    let pipeline = make_pipeline(vec![&src, &conv, &sink])?;
    info!("Pipeline created successfully");
    Element::link_many([&src, &conv, &sink])?;
    info!("Elements linked successfully");

    let Some(bus) = pipeline.bus() else {
        error!("Failed to get pipeline bus");
        return Err(anyhow!("Failed to get pipeline bus"));
    };

    if pipeline.set_state(gst::State::Playing).is_ok() {
        info!("Pipeline set to Playing state");
    } else {
        error!("Failed to set pipeline to Playing state");
        return Err(anyhow!("Failed to set pipeline to Playing state"));
    }

    for msg in bus.iter_timed(gst::ClockTime::NONE) {
        use gst::MessageView;
        match msg.view() {
            MessageView::Eos(..) => {
                info!("End of stream reached.");
                break;
            }
            MessageView::Error(err) => {
                let err_msg = err.error().to_string();

                if err_msg.contains("Output window was closed") {
                    info!("Video window closed by user. Shutting down...");
                } else {
                    error!(
                        "Error from {:?}: {} ({:?})",
                        err.src().map(|s| s.path_string()),
                        err_msg,
                        err.debug()
                    );
                }
                break;
            }
            _ => (),
        }
    }

    if pipeline.set_state(gst::State::Null).is_ok() {
        info!("Pipeline set to Null state");
    } else {
        error!("Failed to set pipeline to Null state");
        return Err(anyhow!("Failed to set pipeline to Null state"));
    }

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

fn make_element(title: &str, properties: Option<Vec<(&str, &str)>>) -> Result<Element> {
    let _ = match ElementFactory::find(title) {
        Some(factory) => factory,
        None => {
            error!("Element factory for '{}' not found", title);
            return Err(anyhow!("Element factory for '{}' not found", title));
        }
    };

    let built = match ElementFactory::make(title).build() {
        Ok(element) => element,
        Err(err) => {
            error!("Failed to create element '{}': {}", title, err);
            return Err(anyhow!("Failed to create element '{}': {}", title, err));
        }
    };

    if let Some(props) = properties {
        for (name, _) in &props {
            if built.find_property(name).is_none() {
                error!("Property '{}' not found for element '{}'", name, title);
                return Err(anyhow!(
                    "Property '{}' not found for element '{}'",
                    name,
                    title
                ));
            }
        }
        for (name, value) in props {
            built.set_property_from_str(name, value);
        }
    }
    Ok(built)
}

fn make_pipeline(elements: Vec<&Element>) -> Result<Pipeline> {
    let pipeline = Pipeline::new();
    if let Err(err) = pipeline.add_many(elements) {
        error!("Failed to add elements to pipeline: {}", err);
        return Err(anyhow!("Failed to add elements to pipeline: {}", err));
    };
    Ok(pipeline)
}
