mod log_parser;
mod streamer;

use std::collections::HashMap;
use std::time::Duration;

use bollard::Docker;
use bollard::container::ListContainersOptions;
use bollard::system::EventsOptions;
use futures_util::StreamExt;
use serde::Deserialize;
use storage::rocks_store::RocksStore;
use tokio::task::JoinHandle;

const DEFAULT_TAIL: usize = 100;
const RECONNECT_INTERVAL: Duration = Duration::from_secs(5);
const CONTAINER_ID_SHORT_LEN: usize = 12;

#[derive(Deserialize, Clone)]
pub struct ContainerConfig {
    pub name: String,
    pub service: Option<String>,
    pub environment: Option<String>,
    pub tail: Option<usize>,
}

pub async fn run(socket: &str, containers_cfg: Vec<ContainerConfig>, store: RocksStore) {
    let name_index: HashMap<String, ContainerConfig> = containers_cfg
        .into_iter()
        .map(|c| (c.name.clone(), c))
        .collect();

    loop {
        let docker = match connect(socket).await {
            Some(d) => d,
            None => {
                tokio::time::sleep(RECONNECT_INTERVAL).await;
                continue;
            }
        };

        let mut tasks: HashMap<String, JoinHandle<()>> = HashMap::new();

        attach_configured_containers(&docker, &name_index, &store, &mut tasks).await;
        watch_events(&docker, &name_index, &store, &mut tasks).await;

        for (_, handle) in tasks.drain() {
            handle.abort();
        }

        tracing::warn!("Docker event stream lost, reconnecting in {RECONNECT_INTERVAL:?}");
        tokio::time::sleep(RECONNECT_INTERVAL).await;
    }
}

async fn connect(socket: &str) -> Option<Docker> {
    let docker = match Docker::connect_with_unix(socket, 120, bollard::API_DEFAULT_VERSION) {
        Ok(d) => d,
        Err(e) => {
            tracing::error!("Failed to connect to Docker at {socket}: {e}");
            return None;
        }
    };

    if let Err(e) = docker.ping().await {
        tracing::error!("Docker ping failed: {e}");
        return None;
    }

    tracing::info!("Docker collector connected via {socket}");
    Some(docker)
}

async fn attach_configured_containers(
    docker: &Docker,
    name_index: &HashMap<String, ContainerConfig>,
    store: &RocksStore,
    tasks: &mut HashMap<String, JoinHandle<()>>,
) {
    let opts = ListContainersOptions::<String> {
        all: true,
        ..Default::default()
    };

    let containers = match docker.list_containers(Some(opts)).await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Failed to list containers: {e}");
            return;
        }
    };

    for container in &containers {
        try_attach_container(docker, container, name_index, store, tasks).await;
    }

    for name in name_index.keys() {
        let found = containers
            .iter()
            .any(|c| extract_container_name(c) == *name);
        if !found {
            tracing::info!("Container {name} not found in Docker, waiting for start event");
        }
    }
}

async fn try_attach_container(
    docker: &Docker,
    container: &bollard::models::ContainerSummary,
    name_index: &HashMap<String, ContainerConfig>,
    store: &RocksStore,
    tasks: &mut HashMap<String, JoinHandle<()>>,
) {
    let Some(id) = &container.id else { return };
    let container_name = extract_container_name(container);
    let Some(cfg) = name_index.get(&container_name) else {
        return;
    };

    let is_running = container.state.as_deref().is_some_and(|s| s == "running");
    if !is_running {
        tracing::info!("Container {container_name} is not running, waiting for start event");
        return;
    }
    if tasks.contains_key(id.as_str()) {
        return;
    }

    let labels = build_labels(cfg, &container_name, id, container);
    let tail = cfg.tail.unwrap_or(DEFAULT_TAIL);
    tracing::info!("Attaching to container {container_name} ({id})");

    let handle = tokio::spawn(streamer::stream_container(
        docker.clone(),
        id.clone(),
        container_name,
        labels,
        store.clone(),
        tail,
    ));
    tasks.insert(id.clone(), handle);
}

async fn watch_events(
    docker: &Docker,
    name_index: &HashMap<String, ContainerConfig>,
    store: &RocksStore,
    tasks: &mut HashMap<String, JoinHandle<()>>,
) {
    let opts = EventsOptions::<String> {
        filters: HashMap::from([
            ("type".to_string(), vec!["container".to_string()]),
            (
                "event".to_string(),
                vec!["start".to_string(), "die".to_string()],
            ),
        ]),
        ..Default::default()
    };

    let mut stream = docker.events(Some(opts));

    while let Some(result) = stream.next().await {
        let event = match result {
            Ok(e) => e,
            Err(e) => {
                tracing::error!("Docker events error: {e}");
                return;
            }
        };
        handle_event(docker, &event, name_index, store, tasks).await;
    }
}

async fn handle_event(
    docker: &Docker,
    event: &bollard::models::EventMessage,
    name_index: &HashMap<String, ContainerConfig>,
    store: &RocksStore,
    tasks: &mut HashMap<String, JoinHandle<()>>,
) {
    let action = event.action.as_deref().unwrap_or("");
    let container_id = match &event.actor {
        Some(actor) => actor.id.as_deref().unwrap_or(""),
        None => return,
    };
    if container_id.is_empty() {
        return;
    }

    match action {
        "start" => handle_start(docker, container_id, name_index, store, tasks).await,
        "die" => {
            if let Some(handle) = tasks.remove(container_id) {
                tracing::info!("Container stopped: {container_id}");
                handle.abort();
            }
        }
        _ => {}
    }
}

async fn handle_start(
    docker: &Docker,
    container_id: &str,
    name_index: &HashMap<String, ContainerConfig>,
    store: &RocksStore,
    tasks: &mut HashMap<String, JoinHandle<()>>,
) {
    if tasks.contains_key(container_id) {
        return;
    }

    let opts = ListContainersOptions {
        filters: HashMap::from([("id".to_string(), vec![container_id.to_string()])]),
        ..Default::default()
    };

    let containers = match docker.list_containers(Some(opts)).await {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("Failed to inspect container {container_id}: {e}");
            return;
        }
    };

    let Some(container) = containers.first() else {
        return;
    };

    let container_name = extract_container_name(container);

    let Some(cfg) = name_index.get(&container_name) else {
        return;
    };

    let labels = build_labels(cfg, &container_name, container_id, container);
    let tail = cfg.tail.unwrap_or(DEFAULT_TAIL);
    tracing::info!("Container started: {container_name} ({container_id})");

    let handle = tokio::spawn(streamer::stream_container(
        docker.clone(),
        container_id.to_string(),
        container_name,
        labels,
        store.clone(),
        tail,
    ));

    tasks.insert(container_id.to_string(), handle);
}

fn extract_container_name(container: &bollard::models::ContainerSummary) -> String {
    container
        .names
        .as_ref()
        .and_then(|n| n.first())
        .map(|n| n.trim_start_matches('/').to_string())
        .unwrap_or_default()
}

fn build_labels(
    cfg: &ContainerConfig,
    container_name: &str,
    container_id: &str,
    container: &bollard::models::ContainerSummary,
) -> HashMap<String, String> {
    let short_id = &container_id[..container_id.len().min(CONTAINER_ID_SHORT_LEN)];

    let service = cfg.service.as_deref().unwrap_or(container_name);

    let mut labels = HashMap::new();
    labels.insert("service".to_string(), service.to_string());
    labels.insert("container_id".to_string(), short_id.to_string());
    labels.insert("container_name".to_string(), container_name.to_string());
    labels.insert(
        "image".to_string(),
        container.image.as_deref().unwrap_or("").to_string(),
    );

    if let Some(env) = &cfg.environment {
        labels.insert("environment".to_string(), env.clone());
    }

    labels
}
