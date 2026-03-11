use anyhow::{Result, anyhow};
use ashpd::desktop::PersistMode;
use ashpd::desktop::screencast::{CursorMode, Screencast, SelectSourcesOptions, SourceType};
use gst::prelude::*;
use gst::{
    Element, ElementFactory, Pipeline, glib::object::ObjectExt, prelude::GObjectExtManualGst,
};
use std::os::fd::{AsRawFd, OwnedFd};
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();
    init_gst()?;
    info!("GStreamer initialized successfully");

    let (node_id, owned_fd) = screencast_source().await?;
    let fd = owned_fd.as_raw_fd();

    let fd_str = fd.to_string();
    let node_id_str = node_id.to_string();

    let src = make_element(
        "pipewiresrc",
        Some(vec![
            ("fd", fd_str.as_str()),
            ("path", node_id_str.as_str()),
        ]),
    )?;
    info!("Source element created successfully");

    let conv = make_element("videoconvert", None)?;
    info!("Video converter element created successfully");

    let enc = make_element(
        "x264enc",
        Some(vec![
            ("tune", "zerolatency"),
            ("speed-preset", "ultrafast"),
            ("key-int-max", "30"),
            ("bitrate", "2000"),
        ]),
    )?;
    info!("H264 encoder element created successfully");

    let enc_caps = make_element(
        "capsfilter",
        Some(vec![(
            "caps",
            "video/x-h264,profile=baseline,stream-format=byte-stream",
        )]),
    )?;
    info!("Encoder caps filter element created successfully");

    let sink = make_element("webrtcsink", None)?;
    info!("WebRTC sink element created successfully");

    let pipeline = make_pipeline(vec![&src, &conv, &enc, &enc_caps, &sink])?;
    info!("Pipeline created successfully");

    Element::link_many([&src, &conv, &enc, &enc_caps, &sink])?;
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
                error!(
                    "Error from {:?}: {} ({:?})",
                    err.src().map(|s| s.path_string()),
                    err_msg,
                    err.debug()
                );
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

async fn screencast_source() -> Result<(u32, OwnedFd)> {
    let proxy = Screencast::new().await?;
    let session = proxy.create_session(Default::default()).await?;

    proxy
        .select_sources(
            &session,
            SelectSourcesOptions::default()
                .set_cursor_mode(CursorMode::Hidden)
                .set_sources(SourceType::Monitor | SourceType::Window)
                .set_multiple(false)
                .set_persist_mode(PersistMode::ExplicitlyRevoked),
        )
        .await?;

    let response = proxy
        .start(&session, None, Default::default())
        .await?
        .response()?;

    info!("Screencast started successfully: {:?}", response);

    let stream = match response.streams().first() {
        Some(stream) => stream,
        None => {
            error!("No streams available in screencast response");
            return Err(anyhow!("No streams available in screencast response"));
        }
    };

    let node_id = stream.pipe_wire_node_id();
    info!("PipeWire node ID: {}", node_id);

    let owned_fd = proxy
        .open_pipe_wire_remote(&session, Default::default())
        .await?;
    info!("PipeWire remote FD opened: {:?}", owned_fd);

    Ok((node_id, owned_fd))
}
