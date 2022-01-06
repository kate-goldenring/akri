pub mod config_action;
pub mod constants;
pub mod crictl_containers;
mod device_plugin_builder;
mod device_plugin_service;
pub mod discovery_operator;
#[cfg(any(test, feature = "agent-full"))]
pub mod embedded_discovery_handlers;
// TODO: more specific visibility
pub mod fs_watch;
pub mod registration;
pub mod slot_reconciliation;
pub mod streaming_extension;
mod v1beta1;
