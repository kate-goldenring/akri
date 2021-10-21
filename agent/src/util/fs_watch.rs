/// Watches for changes to broker management results directory.
/// A subdirectory of this directory is mounted into each management broker Pod for
/// communicating results and status to the Agent and cluster.
extern crate notify;
use akri_shared::k8s::pod::AKRI_JOB_ACTUAL_STATE_LABEL;
use akri_shared::k8s::{KubeImpl, KubeInterface};
use log::{error, info, trace};
use notify::event::{CreateKind, Event, EventKind, ModifyKind};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs::{self, File};
use tokio::io::AsyncReadExt;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};

pub const MANAGEMENT_DIR: &str = "/var/lib/akri/management/";
pub const STATE_FILE: &str = "state.txt";
struct FileSystemWatcher {
    recv: UnboundedReceiver<Event>,
    _watcher: RecommendedWatcher, // holds on to the watcher so it doesn't get dropped
    dir: PathBuf,
}

// Inspired by https://github.com/krustlet/krustlet/blob/main/crates/kubelet/src/fs_watch/mod.rs#L20
impl FileSystemWatcher {
    fn new<P: AsRef<Path>>(dir_to_watch: P) -> anyhow::Result<Self> {
        println!("start watcher");
        let (tx, rx) = unbounded_channel::<Event>();

        // Create a watcher object, delivering debounced events.
        // TODO: consider using RawWatcher and RawEvent instead of Debounced
        let mut watcher = notify::recommended_watcher(move |res| match res {
            Ok(event) => {
                if let Err(e) = tx.send(event) {
                    println!("Unable to send notify event: {:?}", e);
                }
                println!("Send event");
            }
            Err(e) => println!("Notify watch error: {:?}", e),
        })?;

        // Specify management path to be watched. All files and directories at that path and
        // below will be monitored for changes.
        watcher.watch(dir_to_watch.as_ref(), RecursiveMode::Recursive)?;
        println!("watcher returning");
        Ok(FileSystemWatcher {
            recv: rx,
            _watcher: watcher,
            dir: dir_to_watch.as_ref().to_path_buf(),
        })
    }
    // Watches for changes to state.txt files and injects any changed value into instance
    // under AKRI_JOB_ACTUAL_STATE_LABEL
    async fn handle_device_state_change(
        &mut self,
        kube_interface: impl KubeInterface,
    ) -> anyhow::Result<()> {
        loop {
            let event = self.recv.recv().await.expect("Watcher dropped");
            if event.kind.is_modify() {
                match get_state_change_events(&self.dir, event).await? {
                    Some(instance_state_map) => {
                        println!("instance state map is {:?}", instance_state_map);
                        let futures: Vec<_> = instance_state_map
                            .iter()
                            .map(|(instance_name, state)| {
                                update_instance_state(&kube_interface, instance_name, state)
                            })
                            .collect();
                        // Ensure this errors if one errors as described here:
                        // https://docs.rs/futures-util-preview/0.2.2/futures_util/future/fn.join_all.html
                        futures_util::future::join_all(futures).await;
                    }
                    None => continue,
                }
            }
        }
        Ok(())
    }
}

pub async fn do_fs_watch() -> anyhow::Result<()> {
    let management_dir = Path::new(MANAGEMENT_DIR);
    fs::create_dir_all(&management_dir).await.unwrap();
    let kube_interface = KubeImpl::new().await?;
    let mut fs_watcher = FileSystemWatcher::new(management_dir)?;
    fs_watcher.handle_device_state_change(kube_interface).await
}

// TODO: make a method of FileWatcher
async fn update_instance_state(
    kube_interface: &impl KubeInterface,
    instance_name: &str,
    new_state: &str,
) -> anyhow::Result<()> {
    if new_state.is_empty() {
        println!("empty");
        return Ok(());
    }
    // TODO: figure out namespace (normally from config)
    let instance_namespace = "default";
    // TODO: do retries
    let mut instance = kube_interface
        .find_instance(instance_name, instance_namespace)
        .await?;
    println!("HERE");
    // TODO: Do check that is SemVer here
    match instance.spec.broker_properties.insert(
        AKRI_JOB_ACTUAL_STATE_LABEL.to_string(),
        new_state.to_string(),
    ) {
        Some(old_state) => {
            if old_state != new_state {
                trace!(
                    "update_instance_state - {} property updated from {} to {}",
                    AKRI_JOB_ACTUAL_STATE_LABEL,
                    old_state,
                    new_state
                );
                // TODO: do retries
                kube_interface
                    .update_instance(&instance.spec, instance_name, instance_namespace)
                    .await?;
            }
            trace!("update_instance_state - state at expected {}", new_state);
        }
        None => {
            kube_interface
                .update_instance(&instance.spec, instance_name, instance_namespace)
                .await?;
            trace!(
                "update_instance_state - {} property updated to {}",
                AKRI_JOB_ACTUAL_STATE_LABEL,
                new_state
            );
        }
    }
    Ok(())
}
// If event is create or modify
// Determine which instance by examining path
// Get contents of file
// Assumes that an instance's management info is written to the directory MANAGEMENT_DIR/instance_name
// TODO: what to do about multiple brokers writing to same dir? Would more than one update at once?
// Gets all events on state files for all instances
async fn get_state_change_events<P: AsRef<Path>>(
    dir_to_watch: P,
    event: Event,
) -> anyhow::Result<Option<HashMap<String, String>>> {
    trace!(
        "get_file_contents - event {:?} occurred on files {:?}",
        event.kind,
        event.paths
    );
    let paths = event.paths;
    get_job_state_file_contents(dir_to_watch, paths).await
}

async fn get_job_state_file_contents<P: AsRef<Path>>(
    dir_to_watch: P,
    paths: Vec<PathBuf>,
) -> anyhow::Result<Option<HashMap<String, String>>> {
    let mut file_map = HashMap::new();
    // Find the file named job_state.txt
    for p in paths {
        // Assumes looking for the format Path::new("/MANAGEMENT_DIR/instance_name/state.txt")
        // Check if is in management directory
        if let Ok(suffix) = p.strip_prefix(dir_to_watch.as_ref()) {
            // Check is a state file
            if suffix.ends_with(STATE_FILE) {
                println!("is state file");
                // Assume one final parent with instance name
                if let Some(parents) = suffix.parent() {
                    if let Some(instance) = parents.file_name() {
                        println!("is instance parent is {:?}", instance);
                        let mut file = File::open(&p).await?;
                        let mut buf = String::new();
                        // TODO: should only read to certain buffer size
                        // TODO: how to handle empty string/bug
                        file.read_to_string(&mut buf).await?;
                        // Only keep first word in file
                        let version: Vec<&str> = buf.split_ascii_whitespace().collect();
                        if version.len() > 0 {
                            file_map.insert(
                                instance.to_owned().into_string().unwrap(),
                                version[0].to_string(),
                            );
                        }
                    }
                }
            }
        };
    }
    if file_map.is_empty() {
        Ok(None)
    } else {
        Ok(Some(file_map))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use akri_shared::{
        akri::instance::{Instance, InstanceSpec},
        k8s::MockKubeInterface,
    };
    use tempfile::tempdir;
    use tokio::io::{self, AsyncWriteExt, BufWriter};

    // from device_plugin_service
    fn configure_find_instance(
        mock: &mut MockKubeInterface,
        result_file: &'static str,
        instance_name: &'static str,
        instance_namespace: &'static str,
    ) {
        mock.expect_find_instance()
            .times(1)
            .withf(move |name: &str, namespace: &str| {
                namespace == instance_namespace && name == instance_name
            })
            .returning(move |_, _| {
                let instance_json =
                    std::fs::read_to_string(result_file).expect("Unable to read file");
                let instance: Instance = serde_json::from_str(&instance_json).unwrap();
                Ok(instance)
            });
    }

    // TODO: figure out how to test this when there are random modified events
    #[tokio::test]
    async fn test_handle_device_state_change() {
        let _ = env_logger::builder().is_test(true).try_init();
        let instance_name = "instance_name";
        let config_namespace = "default";
        // Create a directory inside of `std::env::temp_dir()`
        let dir = tempdir().unwrap();
        let subdir = dir.path().join(instance_name);
        fs::create_dir_all(&subdir).await.unwrap();
        let file_path = subdir.join(STATE_FILE);
        let _file = File::create(&file_path).await.unwrap();
        let mut fs_watcher = FileSystemWatcher::new(&dir).unwrap();
        let mut mock = MockKubeInterface::new();
        configure_find_instance(
            &mut mock,
            "../test/json/local-instance.json",
            instance_name,
            config_namespace,
        );
        mock.expect_update_instance()
            .times(1)
            .withf(move |instance, name, namespace| {
                namespace == config_namespace
                    && name == instance_name
                    && instance
                        .broker_properties
                        .get(AKRI_JOB_ACTUAL_STATE_LABEL)
                        .unwrap()
                        == &"1".to_string()
            })
            .returning(move |_, _, _| Ok(()));
        // let event = tokio::time::timeout(std::time::Duration::from_secs(1), fs_watcher.recv.recv())
        //     .await
        //     .expect("Timed out waiting for event")
        //     .expect("Watcher dropped");
        // assert!(event.kind.is_create());
        // assert_eq!(event.paths[0], file_path);
        tokio::spawn(async move {
            fs_watcher.handle_device_state_change(mock).await.unwrap();
        });
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        let _w = tokio::fs::write(&file_path, "1").await.unwrap();
        // Thread for `handle_device_state_change` should not terminate. If it does, will ca
        // if let Ok(r) = tokio::time::timeout(std::time::Duration::from_secs(1), test_thread).await {
        //     panic!("{:?}", r);
        // }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }

    // Tests that state string is properly parsed
    #[tokio::test]
    async fn test_update_instance_state() {
        let _ = env_logger::builder().is_test(true).try_init();
        let instance_name = "instance_name";
        let config_namespace = "default";
        // Create a directory inside of `std::env::temp_dir()`
        let mut mock = MockKubeInterface::new();
        configure_find_instance(
            &mut mock,
            "../test/json/local-instance.json",
            instance_name,
            config_namespace,
        );
        mock.expect_update_instance()
            .times(1)
            .withf(move |instance, name, namespace| {
                namespace == config_namespace
                    && name == instance_name
                    && instance
                        .broker_properties
                        .get(AKRI_JOB_ACTUAL_STATE_LABEL)
                        .unwrap()
                        == &"1.1.0".to_string()
            })
            .returning(move |_, _, _| Ok(()));
        update_instance_state(&mock, instance_name, "1.1.0")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_start_watcher() {
        let _ = env_logger::builder().is_test(true).try_init();
        let instance_name = "instance_name";
        // Create a directory inside of `std::env::temp_dir()`
        let dir = tempdir().unwrap();
        let subdir = dir.path().join(instance_name);
        fs::create_dir_all(&subdir).await.unwrap();
        let file_path = subdir.join(STATE_FILE);
        let _file = File::create(&file_path).await.unwrap();
        let mut fs_watcher = FileSystemWatcher::new(&dir).unwrap();
        let event = tokio::time::timeout(std::time::Duration::from_secs(1), fs_watcher.recv.recv())
            .await
            .expect("Timed out waiting for event")
            .expect("Watcher dropped");
        assert!(event.kind.is_create());
        assert_eq!(event.paths[0], file_path);

        let _w = tokio::fs::write(&file_path, "1.1").await.unwrap();
        let event2 =
            tokio::time::timeout(std::time::Duration::from_secs(1), fs_watcher.recv.recv())
                .await
                .expect("Timed out waiting for event")
                .expect("Watcher dropped");
        assert!(event2.kind.is_modify());
        assert_eq!(event2.paths[0], file_path);
        dir.close().unwrap();
    }
    #[tokio::test]
    async fn test_get_state_change_events() {
        let _ = env_logger::builder().is_test(true).try_init();
        // Create a directory inside of `std::env::temp_dir()`
        let dir = tempdir().unwrap();
        let instance_name = "instance_name";
        let subdir = dir.path().join(instance_name);
        fs::create_dir_all(&subdir).await.unwrap();
        let file_path = subdir.join(STATE_FILE);
        let _file = File::create(&file_path).await.unwrap();
        let mut fs_watcher = FileSystemWatcher::new(&dir).unwrap();
        let _w = tokio::fs::write(&file_path, "1.1").await.unwrap();
        let event = tokio::time::timeout(std::time::Duration::from_secs(1), fs_watcher.recv.recv())
            .await
            .expect("Timed out waiting for event")
            .expect("Watcher dropped");
        assert_eq!(
            get_state_change_events(&dir, event)
                .await
                .unwrap()
                .unwrap()
                .get(instance_name)
                .unwrap(),
            "1.1"
        );
        dir.close().unwrap();
    }

    #[tokio::test]
    async fn test_get_job_state_file_contents() {
        // Create a directory inside of `std::env::temp_dir()`
        let dir = tempdir().unwrap();
        let instance_name = "instance_name";
        let subdir = dir.path().join(instance_name);
        fs::create_dir_all(&subdir).await.unwrap();
        let file_path = subdir.join(STATE_FILE);
        println!("dir is {:?}", file_path);

        let mut file = File::create(&file_path).await.unwrap();
        {
            let mut writer = BufWriter::new(file);

            // Write a byte to the buffer.
            writer.write(b"1.1").await.unwrap();

            // Flush the buffer before it goes out of scope.
            writer.flush().await.unwrap();
        }
        let res = get_job_state_file_contents(&dir, vec![file_path])
            .await
            .unwrap();
        // drop(file);
        dir.close().unwrap();
        assert_eq!(res.unwrap().get(instance_name).unwrap(), "1.1");
    }
}
