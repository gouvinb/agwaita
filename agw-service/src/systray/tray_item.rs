//! StatusNotifierItem proxy implementation
//!
//! Provides D-Bus proxy interface for org.kde.StatusNotifierItem

use zbus::{
    proxy,
    zvariant::{
        OwnedObjectPath,
        Type,
    },
};

/// Icon pixmap data from StatusNotifierItem.
#[derive(Clone, Debug, zbus::zvariant::Value, Type, serde::Deserialize, serde::Serialize)]
#[zvariant(signature = "(iiay)")]
pub struct Icon {
    pub width: i32,
    pub height: i32,
    pub bytes: Vec<u8>,
}

/// Tooltip structure for StatusNotifierItem.
#[derive(Clone, Debug, zbus::zvariant::Value, Type, serde::Deserialize, serde::Serialize)]
#[zvariant(signature = "(sa(iiay)ss)")]
pub struct Tooltip {
    pub icon_name: String,
    pub icon_pixmap: Vec<Icon>,
    pub title: String,
    pub description: String,
}

/// D-Bus proxy for org.kde.StatusNotifierItem interface.
#[proxy(interface = "org.kde.StatusNotifierItem")]
pub trait StatusNotifierItem {
    #[zbus(property)]
    fn id(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn category(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn title(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn status(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn icon_name(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn icon_pixmap(&self) -> zbus::Result<Vec<Icon>>;

    #[zbus(property)]
    fn icon_theme_path(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn attention_icon_name(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn attention_icon_pixmap(&self) -> zbus::Result<Vec<Icon>>;

    #[zbus(property)]
    fn overlay_icon_name(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn overlay_icon_pixmap(&self) -> zbus::Result<Vec<Icon>>;

    // #[zbus(property)]
    // fn tool_tip(&self) -> zbus::Result<Tooltip>;

    #[zbus(property)]
    fn item_is_menu(&self) -> zbus::Result<bool>;

    #[zbus(property)]
    fn menu(&self) -> zbus::Result<OwnedObjectPath>;

    // Methods
    fn activate(&self, x: i32, y: i32) -> zbus::Result<()>;

    fn secondary_activate(&self, x: i32, y: i32) -> zbus::Result<()>;

    fn scroll(&self, delta: i32, orientation: &str) -> zbus::Result<()>;

    #[zbus(name = "ContextMenu")]
    fn context_menu(&self, x: i32, y: i32) -> zbus::Result<()>;

    // Signals
    #[zbus(signal)]
    fn new_title(&self) -> zbus::Result<()>;

    #[zbus(signal)]
    fn new_icon(&self) -> zbus::Result<()>;

    #[zbus(signal)]
    fn new_attention_icon(&self) -> zbus::Result<()>;

    #[zbus(signal)]
    fn new_overlay_icon(&self) -> zbus::Result<()>;

    #[zbus(signal)]
    fn new_tool_tip(&self) -> zbus::Result<()>;

    #[zbus(signal)]
    fn new_status(&self, status: String) -> zbus::Result<()>;
}

/// Category of the StatusNotifierItem.
#[derive(Clone, Debug, PartialEq)]
pub enum Category {
    ApplicationStatus,
    Communications,
    SystemServices,
    Hardware,
}

impl TryFrom<&str> for Category {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "ApplicationStatus" => Ok(Category::ApplicationStatus),
            "Communications" => Ok(Category::Communications),
            "SystemServices" => Ok(Category::SystemServices),
            "Hardware" => Ok(Category::Hardware),
            _ => Err(format!("Unknown category: {}", value)),
        }
    }
}

/// Status of the StatusNotifierItem.
#[derive(Clone, Debug, PartialEq)]
pub enum Status {
    Passive,
    Active,
    NeedsAttention,
}

impl TryFrom<&str> for Status {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "Passive" => Ok(Status::Passive),
            "Active" => Ok(Status::Active),
            "NeedsAttention" => Ok(Status::NeedsAttention),
            _ => Err(format!("Unknown status: {}", value)),
        }
    }
}
