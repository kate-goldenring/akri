#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    env_logger::try_init()?;
    #[cfg(target_os = "linux")]
    run().await?;
    #[cfg(not(target_os = "linux"))]
    log::warn!("main - udev is only supported on Linux ... exiting");
    Ok(())
}

#[cfg(target_os = "linux")]
pub async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    use akri_discovery_utils::discovery::discovery_handler::{
        run_discovery_handler, REGISTER_AGAIN_CHANNEL_CAPACITY,
    };
    use akri_udev::{discovery_handler::DiscoveryHandlerImpl, DISCOVERY_HANDLER_NAME, SHARED};

    log::info!("run - udev discovery handler started");
    let (register_sender, register_receiver) =
        tokio::sync::mpsc::channel(REGISTER_AGAIN_CHANNEL_CAPACITY);
    let discovery_handler = DiscoveryHandlerImpl::new(Some(register_sender));
    run_discovery_handler(
        discovery_handler,
        register_receiver,
        DISCOVERY_HANDLER_NAME,
        SHARED,
    )
    .await?;
    log::info!("run - udev discovery handler ended");
    Ok(())
}