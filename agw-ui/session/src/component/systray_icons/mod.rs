//! System tray icon management via StatusNotifierItem protocol.

use agw_lib_outcome::error::AgwError;
use agw_service::runtime;
pub use agw_service::systray::{
    DBusMenuProxy,
    Icon,
    Layout,
    StatusNotifierItemProxy,
    StatusNotifierWatcherProxy,
};
use gtk4::prelude::*;
use log::{
    debug,
    error,
    warn,
};
use relm4::{
    ComponentParts,
    ComponentSender,
    SimpleComponent,
    gtk,
};
use tokio::sync::oneshot;
use zbus::{
    fdo::DBusProxy,
    names::BusName,
};

#[derive(Debug, Clone)]
pub struct StatusNotifierItem {
    pub service_name: String,
    pub display_name: String,
    pub icon_pixbuf: Option<gtk::gdk_pixbuf::Pixbuf>,
    pub icon_name: Option<String>,
    pub layout: Layout,
    #[allow(dead_code)] // public api
    pub item_proxy: StatusNotifierItemProxy<'static>,
    pub menu_proxy: DBusMenuProxy<'static>,
}

#[derive(Clone)]
struct IconData {
    pixmap: Option<Vec<Icon>>,
    icon_name: Option<String>,
    theme_path: Option<String>,
    title: Option<String>,
    id: Option<String>,
}

struct ItemData {
    display_name: String,
    icon_data: IconData,
    layout: Layout,
    item_proxy: StatusNotifierItemProxy<'static>,
    menu_proxy: DBusMenuProxy<'static>,
}

impl StatusNotifierItem {
    pub async fn new(name: String) -> Result<Self, AgwError> {
        let item_data = Self::run_zbus(Self::build_item_data(name.clone()))
            .await
            .ok_or_else(|| AgwError::new(1, "StatusNotifierItem task canceled".to_string()))?
            .map_err(|e| AgwError::new(1, e))?;

        let (icon_pixbuf, icon_name) = Self::resolve_icon(&item_data.icon_data, &name);

        Ok(Self {
            service_name: name,
            display_name: item_data.display_name,
            icon_pixbuf,
            icon_name,
            layout: item_data.layout,
            item_proxy: item_data.item_proxy,
            menu_proxy: item_data.menu_proxy,
        })
    }

    pub async fn update_icon(&mut self) {
        let icon_data = match Self::run_zbus(Self::fetch_icon_data(self.item_proxy.clone())).await {
            Some(Ok(data)) => data,
            Some(Err(e)) => {
                error!(
                    "Failed to refresh icon data for {}: {}",
                    self.service_name, e
                );
                return;
            },
            None => {
                error!("Icon refresh task canceled for {}", self.service_name);
                return;
            },
        };
        let (icon_pixbuf, icon_name) = Self::resolve_icon(&icon_data, &self.service_name);
        self.icon_pixbuf = icon_pixbuf;
        self.icon_name = icon_name;
    }

    fn resolve_icon(icon_data: &IconData, service_name: &str) -> (Option<gtk::gdk_pixbuf::Pixbuf>, Option<String>) {
        if let Some(pixbuf) = icon_data
            .pixmap
            .clone()
            .and_then(Self::sni_pixmap_to_pixbuf)
        {
            return (Some(pixbuf), None);
        }

        let icon_theme = gtk::IconTheme::for_display(&gtk::gdk::Display::default().expect("Could not get default display"));

        if let Some(theme_path) = icon_data.theme_path.as_ref().filter(|s| !s.is_empty()) {
            debug!("Adding custom icon theme path: {}", theme_path);
            icon_theme.add_search_path(theme_path);
        }

        if let Some(name) = icon_data.icon_name.as_ref() {
            // Check if it's an absolute path
            if name.starts_with('/') {
                if let Ok(pixbuf) = gtk::gdk_pixbuf::Pixbuf::from_file(&name) {
                    debug!("Icon loaded from absolute path: {}", name);
                    return (Some(pixbuf), None);
                }
            }

            if icon_theme.has_icon(&name) {
                debug!("Icon found via icon_name: {}", name);
                return (None, Some(name.to_string()));
            }
            warn!("Icon '{}' not found in theme", name);
        }

        // 3. Fallback to title, id or service name
        if let Some(name) = icon_data.title.as_ref() {
            if icon_theme.has_icon(&name) {
                debug!("Icon found via title: {}", name);
                return (None, Some(name.clone()));
            }
        }

        if let Some(name) = icon_data.id.as_ref() {
            if icon_theme.has_icon(&name) {
                debug!("Icon found via id: {}", name);
                return (None, Some(name.clone()));
            }
        }

        if icon_theme.has_icon(service_name) {
            debug!("Icon found via service name: {}", service_name);
            return (None, Some(service_name.to_string()));
        }

        // 4. Final fallback
        debug!(
            "No icon found for {}, using application-x-executable",
            service_name
        );
        (None, Some("application-x-executable".to_string()))
    }

    async fn build_item_data(name: String) -> Result<ItemData, String> {
        let conn = zbus::Connection::session()
            .await
            .map_err(|e| format!("Failed to connect to session bus: {}", e))?;
        let (dest, path) = if let Some(idx) = name.find('/') {
            (&name[..idx], &name[idx..])
        } else {
            (name.as_ref(), "/StatusNotifierItem")
        };

        let item_proxy = StatusNotifierItemProxy::builder(&conn)
            .destination(dest.to_owned())
            .map_err(|e| format!("Invalid D-Bus destination '{}': {}", dest, e))?
            .path(path.to_owned())
            .map_err(|e| format!("Invalid D-Bus path '{}': {}", path, e))?
            .build()
            .await
            .map_err(|e| format!("Failed to create item proxy for {}: {}", name, e))?;

        debug!("item_proxy created for {}", name);

        let menu_path = item_proxy
            .menu()
            .await
            .map_err(|e| format!("Failed to get menu path for {}: {}", name, e))?;
        let menu_proxy = DBusMenuProxy::builder(&conn)
            .destination(dest.to_owned())
            .map_err(|e| format!("Invalid menu destination '{}': {}", dest, e))?
            .path(menu_path.to_owned())
            .map_err(|e| format!("Invalid menu path '{}': {}", menu_path, e))?
            .build()
            .await
            .map_err(|e| format!("Failed to create menu proxy for {}: {}", name, e))?;

        let (_, menu) = menu_proxy
            .get_layout(0, -1, &[])
            .await
            .map_err(|e| format!("Failed to get menu layout for {}: {}", name, e))?;

        let display_name = Self::display_name(&item_proxy, &name).await;
        let icon_data = Self::fetch_icon_data_inner(&item_proxy).await;

        Ok(ItemData {
            display_name,
            icon_data,
            layout: menu,
            item_proxy,
            menu_proxy,
        })
    }

    async fn fetch_icon_data(item_proxy: StatusNotifierItemProxy<'static>) -> Result<IconData, String> {
        Ok(Self::fetch_icon_data_inner(&item_proxy).await)
    }

    async fn fetch_icon_data_inner(item_proxy: &StatusNotifierItemProxy<'static>) -> IconData {
        let status = item_proxy
            .status()
            .await
            .ok()
            .unwrap_or_else(|| "Active".to_string());
        let is_attention = status == "NeedsAttention";

        let pixmap = if is_attention {
            item_proxy
                .attention_icon_pixmap()
                .await
                .ok()
                .or(item_proxy.icon_pixmap().await.ok())
        } else {
            item_proxy.icon_pixmap().await.ok()
        };

        let icon_name = if is_attention {
            item_proxy
                .attention_icon_name()
                .await
                .ok()
                .filter(|s| !s.is_empty())
                .or(item_proxy.icon_name().await.ok().filter(|s| !s.is_empty()))
        } else {
            item_proxy.icon_name().await.ok().filter(|s| !s.is_empty())
        };

        let theme_path = item_proxy
            .icon_theme_path()
            .await
            .ok()
            .filter(|s| !s.is_empty());
        let title = item_proxy.title().await.ok().filter(|s| !s.is_empty());
        let id = item_proxy.id().await.ok().filter(|s| !s.is_empty());

        IconData {
            pixmap,
            icon_name,
            theme_path,
            title,
            id,
        }
    }

    async fn display_name(item_proxy: &StatusNotifierItemProxy<'static>, name: &str) -> String {
        let id = item_proxy.id().await.ok();
        let title = item_proxy.title().await.ok();

        id.filter(|s| !s.is_empty())
            .or_else(|| title.filter(|s| !s.is_empty()))
            .unwrap_or_else(|| name.to_string())
    }

    async fn run_zbus<T, F>(future: F) -> Option<T>
    where
        F: std::future::Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        run_zbus_task(future).await
    }

    fn sni_pixmap_to_pixbuf(icons: Vec<Icon>) -> Option<gtk::gdk_pixbuf::Pixbuf> {
        if icons.is_empty() {
            return None;
        }
        icons
            .into_iter()
            .max_by_key(|i| (i.width, i.height))
            .map(|mut i| {
                // Convert ARGB (StatusNotifierItem format) to RGBA (GTK format)
                for pixel in i.bytes.chunks_exact_mut(4) {
                    pixel.rotate_left(1);
                }
                gtk::gdk_pixbuf::Pixbuf::from_mut_slice(
                    i.bytes,
                    gtk::gdk_pixbuf::Colorspace::Rgb,
                    true,
                    8,
                    i.width,
                    i.height,
                    i.width * 4,
                )
            })
    }
}

async fn run_zbus_task<T, F>(future: F) -> Option<T>
where
    F: std::future::Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    let (sender, receiver) = oneshot::channel();
    runtime::spawn(async move {
        let _ = sender.send(future.await);
    });
    receiver.await.ok()
}

pub async fn is_name_active(name: &str) -> bool {
    let name = name.to_string();
    run_zbus_task(async move {
        let conn = match zbus::Connection::session().await {
            Ok(conn) => conn,
            Err(e) => {
                error!("Failed to connect to session bus for NameHasOwner: {}", e);
                return false;
            },
        };

        let bus_name = match BusName::try_from(name.as_str()) {
            Ok(name) => name,
            Err(e) => {
                error!("Invalid D-Bus name for NameHasOwner '{}': {}", name, e);
                return false;
            },
        };

        let dbus = match DBusProxy::new(&conn).await {
            Ok(proxy) => proxy,
            Err(e) => {
                error!("Failed to create DBusProxy for NameHasOwner: {}", e);
                return false;
            },
        };

        dbus.name_has_owner(bus_name).await.unwrap_or(false)
    })
    .await
    .unwrap_or(false)
}

pub async fn watcher_register_host(watcher: StatusNotifierWatcherProxy<'static>, service: &str) {
    let service = service.to_string();
    let _ = run_zbus_task(async move { watcher.register_status_notifier_host(&service).await });
}

pub async fn watcher_registered_items(watcher: StatusNotifierWatcherProxy<'static>) -> Vec<String> {
    run_zbus_task(async move {
        watcher
            .registered_status_notifier_items()
            .await
            .unwrap_or_default()
    })
    .await
    .unwrap_or_default()
}

pub struct SystemTrayIcons {
    items: Vec<StatusNotifierItem>,
    container: gtk::Box,
}

#[derive(Debug)]
pub enum SystemTrayIconsInput {
    ItemRegistered(StatusNotifierItem),
    ItemUnregistered(String),
    IconChanged(String),
    StatusChanged(String, String),
    MenuLayoutChanged(String, Layout),
}

impl Clone for SystemTrayIconsInput {
    fn clone(&self) -> Self {
        match self {
            SystemTrayIconsInput::ItemRegistered(item) => SystemTrayIconsInput::ItemRegistered(item.clone()),
            SystemTrayIconsInput::ItemUnregistered(name) => SystemTrayIconsInput::ItemUnregistered(name.clone()),
            SystemTrayIconsInput::IconChanged(name) => SystemTrayIconsInput::IconChanged(name.clone()),
            SystemTrayIconsInput::StatusChanged(name, status) => SystemTrayIconsInput::StatusChanged(name.clone(), status.clone()),
            SystemTrayIconsInput::MenuLayoutChanged(name, layout) => SystemTrayIconsInput::MenuLayoutChanged(name.clone(), layout.clone()),
        }
    }
}

#[relm4::component(pub)]
impl SimpleComponent for SystemTrayIcons {
    type Init = ();
    type Input = SystemTrayIconsInput;
    type Output = ();

    view! {
        #[root]
        gtk::Box {
            set_spacing: 4,
            set_halign: gtk::Align::End,
        }
    }

    fn init(_init: Self::Init, root: Self::Root, _sender: ComponentSender<Self>) -> ComponentParts<Self> {
        debug!("Initializing SystemTrayIcons component");

        let css_provider = gtk::CssProvider::new();
        css_provider.load_from_string("popover.menu.no-toggles box.vertical > modelbutton { padding-left: 6px; }");

        if let Some(display) = gtk::gdk::Display::default() {
            gtk::style_context_add_provider_for_display(
                &display,
                &css_provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }

        let model = SystemTrayIcons {
            items: Vec::new(),
            container: root.clone(),
        };

        let widgets = view_output!();

        // NOTE: DBus watcher is started globally in TopbarManager to avoid conflicts
        // when multiple monitors are present. This component only displays the icons.

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {
            SystemTrayIconsInput::ItemRegistered(item) => {
                debug!(
                    "Registering item: service_name={}, display_name={}",
                    item.service_name, item.display_name
                );
                // Check if item already exists (update or push)
                if let Some(existing) = self
                    .items
                    .iter_mut()
                    .find(|i| i.service_name == item.service_name)
                {
                    *existing = item;
                } else {
                    self.items.push(item);
                }
                self.refresh_tray_icons();
            },
            SystemTrayIconsInput::ItemUnregistered(service_name) => {
                debug!("Unregistering item: {}", service_name);
                let len_before = self.items.len();
                self.items.retain(|item| item.service_name != service_name);
                if self.items.len() != len_before {
                    self.refresh_tray_icons();
                }
            },
            SystemTrayIconsInput::IconChanged(service_name) => {
                if let Some(item) = self
                    .items
                    .iter_mut()
                    .find(|item| item.service_name == service_name)
                {
                    let mut item_clone = item.clone();
                    let sender = sender.clone();
                    relm4::spawn_local(async move {
                        item_clone.update_icon().await;
                        sender.input(SystemTrayIconsInput::ItemRegistered(item_clone));
                    });
                }
            },
            SystemTrayIconsInput::StatusChanged(service_name, _status) => {
                // For now, just reload the icon when status changes (it might change between Active and NeedsAttention)
                sender.input(SystemTrayIconsInput::IconChanged(service_name));
            },
            SystemTrayIconsInput::MenuLayoutChanged(service_name, layout) => {
                if let Some(item) = self
                    .items
                    .iter_mut()
                    .find(|item| item.service_name == service_name)
                {
                    item.layout = layout;
                }
            },
        }
    }
}

impl SystemTrayIcons {
    /// Check if a layout contains any toggle items (checkboxes or radio buttons).
    fn has_toggle_items(layout: &Layout) -> bool {
        if layout.1.toggle_type.is_some() {
            return true;
        }
        layout.2.iter().any(|child| Self::has_toggle_items(child))
    }

    /// Format a DBusMenu shortcut to GTK accelerator format.
    ///
    /// DBusMenu shortcuts are arrays of arrays: [["Control", "c"], ["Alt", "x"]]
    /// Returns GTK accelerator format like "<Control>c" or "<Alt>x"
    fn format_shortcut(shortcut: &[Vec<String>]) -> Option<String> {
        if shortcut.is_empty() {
            return None;
        }

        // Take the first shortcut combination
        let keys = &shortcut[0];
        if keys.is_empty() {
            return None;
        }

        let mut modifiers = Vec::new();
        let mut base_key = String::new();

        for key in keys {
            match key.as_str() {
                "Control" => modifiers.push("Control"),
                "Shift" => modifiers.push("Shift"),
                "Alt" => modifiers.push("Alt"),
                "Super" => modifiers.push("Super"),
                "Meta" => modifiers.push("Meta"),
                other => base_key = other.to_string(),
            }
        }

        if base_key.is_empty() {
            return None;
        }

        // Build GTK accelerator format: <Control><Shift>a
        let modifier_string = modifiers
            .iter()
            .map(|m| format!("<{}>", m))
            .collect::<String>();
        Some(format!("{}{}", modifier_string, base_key))
    }

    fn refresh_tray_icons(&self) {
        debug!("Refreshing tray icons, count: {}", self.items.len());

        // Remove all existing buttons
        while let Some(child) = self.container.first_child() {
            self.container.remove(&child);
        }

        // Add a MenuButton for each system tray item
        for item in &self.items {
            let menu_button = gtk::MenuButton::new();

            // Set icon from pixbuf or icon name
            if let Some(ref icon_name) = item.icon_name {
                let image = gtk::Image::from_icon_name(icon_name);
                menu_button.set_child(Some(&image));
            } else if let Some(ref pixbuf) = item.icon_pixbuf {
                #[allow(deprecated)]
                let texture = gtk::gdk::Texture::for_pixbuf(pixbuf);
                let image = gtk::Image::from_paintable(Some(&texture));
                menu_button.set_child(Some(&image));
            } else {
                warn!(
                    "No icon available for {}, showing fallback icon",
                    item.display_name
                );
                let image = gtk::Image::from_icon_name("application-x-executable");
                menu_button.set_child(Some(&image));
            }

            // Create popover menu from DBus menu layout with actions
            let popover = self.create_menu_popover(item);
            menu_button.set_popover(Some(&popover));

            menu_button.set_tooltip_text(Some(&item.display_name));
            self.container.append(&menu_button);
            debug!("Added tray icon: {}", item.display_name);
        }
    }

    fn create_menu_popover(&self, item: &StatusNotifierItem) -> gtk::PopoverMenu {
        let menu = gtk::gio::Menu::new();
        let action_group = gtk::gio::SimpleActionGroup::new();

        // Build menu from DBus layout and register actions
        self.build_menu_model(&menu, &action_group, &item.layout, &item.menu_proxy);

        let popover = gtk::PopoverMenu::from_model(Some(&menu));
        popover.insert_action_group("tray", Some(&action_group));

        // Apply CSS to adjust padding only if menu contains toggle items
        if Self::has_toggle_items(&item.layout) {
            popover.add_css_class("has-toggles");
        } else {
            popover.add_css_class("no-toggles");
        }

        popover
    }

    fn build_menu_model(&self, menu: &gtk::gio::Menu, action_group: &gtk::gio::SimpleActionGroup, layout: &Layout, menu_proxy: &DBusMenuProxy<'static>) {
        let mut current_section = gtk::gio::Menu::new();
        let mut has_items_in_section = false;

        // Layout structure: Layout(id, props, children)
        for child in &layout.2 {
            // Check if it's a separator
            if child.1.item_type.as_deref() == Some("separator") {
                // Finalize current section and start a new one
                if has_items_in_section {
                    menu.append_section(None, &current_section);
                    current_section = gtk::gio::Menu::new();
                    has_items_in_section = false;
                }
                continue;
            }

            // Skip items without labels
            if let Some(ref label) = child.1.label {
                if label.is_empty() {
                    continue;
                }

                // If has children, create submenu
                if !child.2.is_empty() {
                    let submenu = gtk::gio::Menu::new();
                    self.build_menu_model(&submenu, action_group, child, menu_proxy);
                    current_section.append_submenu(Some(label), &submenu);
                    has_items_in_section = true;
                } else {
                    let item_id = child.0;
                    let action_name = format!("item-{}", item_id);
                    let is_enabled = child.1.enabled.unwrap_or(true);

                    // Check if this is a toggle item
                    if let Some(ref toggle_type) = child.1.toggle_type {
                        // Create stateful action for toggle items
                        let initial_state = child.1.toggle_state.unwrap_or(0) != 0;
                        let state_variant = initial_state.to_variant();
                        let action = gtk::gio::SimpleAction::new_stateful(&action_name, None, &state_variant);
                        let menu_proxy_clone = menu_proxy.clone();

                        action.connect_activate(move |action, _| {
                            if let Some(state) = action.state() {
                                let current = state.get::<bool>().unwrap_or(false);
                                let new_state = !current;
                                action.set_state(&new_state.to_variant());

                                let menu_proxy = menu_proxy_clone.clone();
                                runtime::spawn(async move {
                                    let value = zbus::zvariant::Value::I32(if new_state { 1 } else { 0 })
                                        .try_to_owned()
                                        .unwrap();
                                    let timestamp = std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap()
                                        .as_secs() as u32;

                                    if let Err(e) = menu_proxy
                                        .event(item_id, "clicked", &value, timestamp)
                                        .await
                                    {
                                        error!("Failed to send menu event: {}", e);
                                    }
                                });
                            }
                        });

                        action.set_enabled(is_enabled);
                        action_group.add_action(&action);

                        let menu_item = gtk::gio::MenuItem::new(Some(label), Some(&format!("tray.{}", action_name)));

                        if toggle_type == "radio" {
                            menu_item.set_attribute_value("role", Some(&toggle_type.to_variant()));
                        }

                        // Add keyboard shortcut if available
                        if let Some(ref shortcuts) = child.1.shortcut {
                            if let Some(formatted) = Self::format_shortcut(shortcuts) {
                                menu_item.set_attribute_value("accel", Some(&formatted.to_variant()));
                            }
                        }

                        current_section.append_item(&menu_item);
                        has_items_in_section = true;
                    } else {
                        // Regular menu item (non-toggle)
                        let action = gtk::gio::SimpleAction::new(&action_name, None);
                        let menu_proxy_clone = menu_proxy.clone();

                        action.connect_activate(move |_, _| {
                            let menu_proxy = menu_proxy_clone.clone();
                            runtime::spawn(async move {
                                let value = zbus::zvariant::Value::I32(0).try_to_owned().unwrap();
                                let timestamp = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs() as u32;

                                if let Err(e) = menu_proxy
                                    .event(item_id, "clicked", &value, timestamp)
                                    .await
                                {
                                    error!("Failed to send menu event: {}", e);
                                }
                            });
                        });

                        action.set_enabled(is_enabled);
                        action_group.add_action(&action);

                        let menu_item = gtk::gio::MenuItem::new(Some(label), Some(&format!("tray.{}", action_name)));

                        // Add keyboard shortcut if available
                        if let Some(ref shortcuts) = child.1.shortcut {
                            if let Some(formatted) = Self::format_shortcut(shortcuts) {
                                menu_item.set_attribute_value("accel", Some(&formatted.to_variant()));
                            }
                        }

                        current_section.append_item(&menu_item);
                        has_items_in_section = true;
                    }
                }
            }
        }

        // Append final section if it has items
        if has_items_in_section {
            menu.append_section(None, &current_section);
        }
    }
}
