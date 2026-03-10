use gtk4::prelude::*;
use log::{
    debug,
    error,
};
use relm4::{
    ComponentParts,
    ComponentSender,
    RelmWidgetExt,
    SimpleComponent,
    gtk,
};
use std::path::Path;

/// AvatarIcon - Displays user avatar image or first letter of username
pub struct AvatarIcon {
    username: String,
    first_letter: String,
    has_avatar: bool,
    avatar_path: String,
    _monitor: Option<AvatarMonitor>,
}

impl AvatarIcon {
    fn check_avatar_exists(path: &str) -> bool {
        Path::new(path).exists()
    }

    fn get_first_letter(username: &str) -> String {
        username
            .chars()
            .next()
            .unwrap_or('?')
            .to_uppercase()
            .to_string()
    }
}

#[derive(Debug)]
pub enum AvatarIconInput {
    AvatarChanged(bool),
}

/// Monitor for avatar file changes using inotify
struct AvatarMonitor {
    _handle: std::thread::JoinHandle<()>,
}

impl AvatarMonitor {
    /// Start monitoring avatar file with inotify
    fn start<F>(path: String, callback: F) -> Self
    where
        F: Fn(bool) + Send + 'static,
    {
        let handle = std::thread::spawn(move || {
            use inotify::{
                Inotify,
                WatchMask,
            };

            match Inotify::init() {
                Ok(mut inotify) => {
                    // Watch parent directory since the avatar file might be created/deleted
                    let parent_dir = Path::new(&path)
                        .parent()
                        .unwrap_or(Path::new("/var/lib/AccountsService/icons"));

                    match inotify.watches().add(
                        parent_dir,
                        WatchMask::CREATE | WatchMask::DELETE | WatchMask::MODIFY | WatchMask::MOVE,
                    ) {
                        Ok(_watch) => {
                            debug!(
                                "Started inotify watch on avatar directory: {:?}",
                                parent_dir
                            );

                            let mut buffer = [0u8; 4096];

                            // Listen for events
                            loop {
                                match inotify.read_events_blocking(&mut buffer) {
                                    Ok(events) => {
                                        for event in events {
                                            // Check if the event is for our avatar file
                                            if let Some(name) = event.name {
                                                if parent_dir.join(name).to_str() == Some(&path) {
                                                    let exists = Path::new(&path).exists();
                                                    debug!("Avatar file changed: exists={}", exists);
                                                    callback(exists);
                                                }
                                            }
                                        }
                                    },
                                    Err(e) => {
                                        error!("Error reading inotify events for avatar: {}", e);
                                        break;
                                    },
                                }
                            }
                        },
                        Err(e) => {
                            error!("Failed to add inotify watch for avatar directory: {}", e);
                        },
                    }
                },
                Err(e) => {
                    error!("Failed to initialize inotify for avatar: {}", e);
                },
            }
        });

        Self { _handle: handle }
    }
}

#[relm4::component(pub)]
impl SimpleComponent for AvatarIcon {
    type Init = ();
    type Input = AvatarIconInput;
    type Output = ();

    view! {
        #[root]
        gtk::Box {
            set_halign: gtk::Align::Center,
            set_valign: gtk::Align::Center,
            set_spacing: 0,

            gtk::Box {
                inline_css: "border-radius: 50%;",
                set_halign: gtk::Align::Center,
                set_valign: gtk::Align::Center,
                set_overflow: gtk::Overflow::Hidden,


                // Avatar image when available
                gtk::Image {
                    set_pixel_size: 16,

                    #[watch]
                    set_visible: model.has_avatar,
                    #[watch]
                    set_from_file: Some(&model.avatar_path),
                },
                // Fallback: first letter in colored circle
                gtk::Label {
                    inline_css: "background-color: var(--accent-bg-color); color: var(--accent-fg-color);",
                    set_halign: gtk::Align::Center,
                    set_valign: gtk::Align::Center,
                    set_height_request: 20,
                    set_width_request: 20,

                    #[watch]
                    set_visible: !model.has_avatar,
                    #[watch]
                    set_label: &model.first_letter,
                },
            },
        }
    }

    fn init(#[allow(unused_variables)] init: Self::Init, root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        // Get current username
        let username = std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .unwrap_or_else(|_| "user".to_string());

        let first_letter = Self::get_first_letter(&username);
        let avatar_path = format!("/var/lib/AccountsService/icons/{}", username);
        let has_avatar = Self::check_avatar_exists(&avatar_path);

        debug!(
            "Avatar initialized: username={}, has_avatar={}, path={}",
            username, has_avatar, avatar_path
        );

        // Start inotify monitor for avatar file changes
        let sender_clone = sender.input_sender().clone();
        let avatar_path_clone = avatar_path.clone();
        let monitor = AvatarMonitor::start(avatar_path_clone, move |exists| {
            sender_clone
                .send(AvatarIconInput::AvatarChanged(exists))
                .ok();
        });

        let model = AvatarIcon {
            username: username.clone(),
            first_letter,
            has_avatar,
            avatar_path: avatar_path.clone(),
            _monitor: Some(monitor),
        };

        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, #[allow(unused_variables)] sender: ComponentSender<Self>) {
        match message {
            AvatarIconInput::AvatarChanged(exists) => {
                if exists != self.has_avatar {
                    self.has_avatar = exists;
                    debug!(
                        "Avatar state changed: has_avatar={} for user {}",
                        self.has_avatar, self.username
                    );
                }
            },
        }
    }
}
