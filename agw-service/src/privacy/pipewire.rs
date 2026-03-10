//! PipeWire monitoring for audio/video recording detection.
//!
//! Adapted from Ashell's implementation using native PipeWire API.
//! Includes webcam monitoring via inotify for legacy applications.

use inotify::{
    EventMask,
    Inotify,
    WatchMask,
};
use log::{
    debug,
    error,
    trace,
    warn,
};
use pipewire::{
    context::ContextBox,
    main_loop::MainLoopBox,
};
use std::{
    collections::HashSet,
    fs,
    path::Path,
    sync::mpsc::{
        Receiver,
        Sender,
        channel,
    },
    thread,
};

const WEBCAM_DEVICE_PATH: &str = "/dev/video0";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Media {
    Video(String),
    Audio(String),
}

#[derive(Debug, Clone)]
pub struct ApplicationNode {
    pub id: u32,
    pub media: Media,
    pub app_name: String,
    pub node_name: String,
    pub media_role: Option<String>,
    pub media_name: Option<String>,
}

#[derive(Debug, Clone)]
pub enum PrivacyEvent {
    AddNode(ApplicationNode),
    RemoveNode(u32),
    WebcamOpen,
    WebcamClose,
}

/// PipeWire monitor for detecting active audio/video recording.
pub struct PipeWireMonitor {
    _pw_thread: Option<thread::JoinHandle<()>>,
    _webcam_thread: Option<thread::JoinHandle<()>>,
    _state_thread: Option<thread::JoinHandle<()>>,
}

impl PipeWireMonitor {
    /// Create a new PipeWire monitor with a callback for usage updates.
    ///
    /// The callback receives (camera_apps, microphone_apps, screencast_apps).
    pub fn new<F>(callback: F) -> Result<Self, Box<dyn std::error::Error>>
    where
        F: FnMut(HashSet<String>, HashSet<String>, HashSet<String>) + Send + 'static,
    {
        let (tx, rx) = channel::<PrivacyEvent>();

        // Spawn PipeWire listener thread
        let pw_tx = tx.clone();
        let pw_thread = thread::spawn(move || {
            if let Err(e) = Self::run_pipewire_listener(pw_tx) {
                error!("PipeWire listener failed: {}", e);
            }
        });

        // Spawn webcam listener thread
        let webcam_tx = tx.clone();
        let webcam_thread = thread::spawn(move || {
            if let Err(e) = Self::run_webcam_listener(webcam_tx) {
                warn!("Webcam listener failed: {}", e);
            }
        });

        // Spawn state manager thread
        let state_thread = thread::spawn(move || {
            Self::run_state_manager(rx, callback);
        });

        Ok(PipeWireMonitor {
            _pw_thread: Some(pw_thread),
            _webcam_thread: Some(webcam_thread),
            _state_thread: Some(state_thread),
        })
    }

    fn run_pipewire_listener(tx: Sender<PrivacyEvent>) -> Result<(), Box<dyn std::error::Error>> {
        pipewire::init();

        let mainloop = MainLoopBox::new(None)?;
        let context = ContextBox::new(mainloop.loop_(), None)?;
        let core = context.connect(None)?;
        let registry = core.get_registry()?;

        let tx_clone = tx.clone();
        let _listener = registry
            .add_listener_local()
            .global(move |global| {
                if let Some(props) = &global.props {
                    if let Some(media_class) = props.get("media.class") {
                        let app_name = props
                            .get("application.name")
                            .or_else(|| props.get("node.name"))
                            .unwrap_or("Unknown")
                            .to_string();

                        let node_name = props.get("node.name").unwrap_or("Unknown").to_string();
                        let media_role = props.get("media.role").map(|role| role.to_string());
                        let media_name = props
                            .get("media.name")
                            .or_else(|| props.get("node.description"))
                            .map(|name| name.to_string());

                        trace!(
                            "New PipeWire node: id={}, media={}, app={}",
                            global.id, media_class, app_name
                        );
                        match media_class {
                            "Stream/Input/Video" | "Stream/Input/Audio" => {
                                if Self::should_include_app(&app_name) {
                                    let _ = tx_clone.send(PrivacyEvent::AddNode(ApplicationNode {
                                        id: global.id,
                                        media: if media_class == "Stream/Input/Video" {
                                            Media::Video(media_class.to_string())
                                        } else {
                                            Media::Audio(media_class.to_string())
                                        },
                                        app_name,
                                        node_name,
                                        media_role,
                                        media_name,
                                    }));
                                }
                            },
                            "Stream/Output/Video" => {
                                if Self::should_include_app(&app_name) {
                                    let _ = tx_clone.send(PrivacyEvent::AddNode(ApplicationNode {
                                        id: global.id,
                                        media: Media::Video(media_class.to_string()),
                                        app_name,
                                        node_name,
                                        media_role,
                                        media_name,
                                    }));
                                }
                            },
                            _ => return,
                        }
                    }
                }
            })
            .global_remove({
                let tx = tx.clone();
                move |id| {
                    debug!("Remove PipeWire node: {}", id);
                    let _ = tx.send(PrivacyEvent::RemoveNode(id));
                }
            })
            .register();

        debug!("PipeWire listener started");
        mainloop.run();

        warn!("PipeWire mainloop exited");
        Ok(())
    }

    fn run_webcam_listener(tx: Sender<PrivacyEvent>) -> Result<(), Box<dyn std::error::Error>> {
        let mut inotify = Inotify::init()?;

        inotify.watches().add(
            WEBCAM_DEVICE_PATH,
            WatchMask::CLOSE_WRITE | WatchMask::CLOSE_NOWRITE | WatchMask::DELETE_SELF | WatchMask::OPEN | WatchMask::ATTRIB,
        )?;

        let mut buffer = [0; 512];
        debug!("Webcam listener started for {}", WEBCAM_DEVICE_PATH);

        loop {
            let events = inotify.read_events_blocking(&mut buffer)?;

            for event in events {
                // trace!("Webcam event: {:?}", event.mask);
                match event.mask {
                    EventMask::OPEN => {
                        let _ = tx.send(PrivacyEvent::WebcamOpen);
                    },
                    EventMask::CLOSE_WRITE | EventMask::CLOSE_NOWRITE => {
                        let _ = tx.send(PrivacyEvent::WebcamClose);
                    },
                    _ => {},
                }
            }
        }
    }

    fn run_state_manager<F>(rx: Receiver<PrivacyEvent>, mut callback: F)
    where
        F: FnMut(HashSet<String>, HashSet<String>, HashSet<String>) + Send + 'static,
    {
        let mut nodes = Vec::<ApplicationNode>::new();
        let mut webcam_access = is_device_in_use(WEBCAM_DEVICE_PATH);
        let mut last_camera = HashSet::new();
        let mut last_mic = HashSet::new();
        let mut last_screencast = HashSet::new();

        debug!("Initial webcam access count: {}", webcam_access);

        while let Ok(event) = rx.recv() {
            match event {
                PrivacyEvent::AddNode(node) => {
                    nodes.push(node);
                },
                PrivacyEvent::RemoveNode(id) => {
                    nodes.retain(|n| n.id != id);
                },
                PrivacyEvent::WebcamOpen => {
                    let new_count = is_device_in_use(WEBCAM_DEVICE_PATH);
                    if new_count != webcam_access {
                        webcam_access = new_count;
                        debug!("Webcam access count updated: {}", webcam_access);
                    }
                },
                PrivacyEvent::WebcamClose => {
                    let new_count = is_device_in_use(WEBCAM_DEVICE_PATH);
                    if new_count != webcam_access {
                        webcam_access = new_count;
                        debug!("Webcam access count updated: {}", webcam_access);
                    }
                },
            }

            let (camera, mic, screencast) = Self::categorize_nodes(&nodes, webcam_access);

            if camera == last_camera && mic == last_mic && screencast == last_screencast {
                continue;
            }

            if !camera.is_empty() || !mic.is_empty() || !screencast.is_empty() {
                debug!(
                    "Privacy update - camera: {}, mic: {}, screencast: {}",
                    camera.len(),
                    mic.len(),
                    screencast.len()
                );
            }

            last_camera = camera.clone();
            last_mic = mic.clone();
            last_screencast = screencast.clone();

            callback(camera, mic, screencast);
        }
    }

    fn categorize_nodes(nodes: &[ApplicationNode], webcam_access: i32) -> (HashSet<String>, HashSet<String>, HashSet<String>) {
        let mut camera_apps = HashSet::new();
        let mut mic_apps = HashSet::new();
        let mut screencast_apps = HashSet::new();

        // Add webcam direct access if detected
        if webcam_access > 0 {
            camera_apps.insert("∙ Webcam device <i>(direct access)</i>".to_string());
        }

        for node in nodes {
            let formatted = Self::format_app_name(&node.app_name, &node.node_name);

            match &node.media {
                Media::Video(media_class) => {
                    if Self::is_screencast(node, media_class) {
                        // trace!("{} is a screencast", node.app_name);
                        screencast_apps.insert(formatted);
                    } else {
                        // trace!("{} is a camera", node.app_name);
                        camera_apps.insert(formatted);
                    }
                },
                Media::Audio(_) => {
                    mic_apps.insert(formatted);
                },
            }
        }

        (camera_apps, mic_apps, screencast_apps)
    }

    fn should_include_app(app_name: &str) -> bool {
        let app_lower = app_name.to_lowercase();
        !app_lower.contains("pipewire")
            && !app_lower.contains("wireplumber")
            && !app_lower.contains("monitor")
            && !app_lower.contains("built-in audio")
            && !app_lower.contains("bluez")
            && app_name != "Unknown"
    }

    fn is_screencast(node: &ApplicationNode, media_class: &str) -> bool {
        let combined = format!(
            "{} {} {} {} {}",
            node.app_name,
            node.node_name,
            media_class,
            node.media_role.as_deref().unwrap_or(""),
            node.media_name.as_deref().unwrap_or("")
        )
        .to_lowercase();
        // trace!("Checking if {} is a screencast", combined);
        combined.contains("webrtc-consume-stream")
            || combined.contains("screen")
            || combined.contains("desktop")
            || combined.contains("capture")
            || combined.contains("share")
            || combined.contains("stream/output/video") // Firefox screen share fix
    }

    fn format_app_name(app_name: &str, node_name: &str) -> String {
        if app_name != node_name && node_name != "Unknown" && !node_name.is_empty() {
            format!("∙ {} <i>({})</i>", app_name, node_name)
        } else {
            format!("∙ {}", app_name)
        }
    }
}

/// Check if a device is currently in use by scanning /proc/*/fd
fn is_device_in_use(target: &str) -> i32 {
    let mut used_by = 0;

    if let Ok(entries) = fs::read_dir("/proc") {
        entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.join("fd").exists())
            .filter_map(|pid_path| fs::read_dir(pid_path.join("fd")).ok())
            .flat_map(|fd_entries| fd_entries.flatten())
            .filter_map(|fd_entry| fs::read_link(fd_entry.path()).ok())
            .filter(|link_path| link_path == Path::new(target))
            .for_each(|_| used_by += 1);
    }

    used_by
}
