//! DBusMenu proxy implementation
//!
//! Provides D-Bus proxy interface for com.canonical.dbusmenu

use zbus::{
    proxy,
    zvariant::{
        OwnedValue,
        Signature,
        Type,
    },
};

/// DBusMenu layout tree structure (id, properties, children).
#[derive(Clone, Debug, Type)]
#[zvariant(signature = "(ia{sv}av)")]
pub struct Layout(pub i32, pub LayoutProps, pub Vec<Layout>);

impl<'a> serde::Deserialize<'a> for Layout {
    fn deserialize<D: serde::Deserializer<'a>>(deserializer: D) -> Result<Self, D::Error> {
        let (id, props, children) = <(i32, LayoutProps, Vec<(Signature, Self)>)>::deserialize(deserializer)?;
        Ok(Self(id, props, children.into_iter().map(|x| x.1).collect()))
    }
}

/// Properties for DBusMenu layout items.
#[derive(Clone, Debug, Type, zbus::zvariant::DeserializeDict)]
#[zvariant(signature = "dict")]
pub struct LayoutProps {
    // #[zvariant(rename = "children-display")]
    // pub children_display: Option<String>,
    pub label: Option<String>,
    pub enabled: Option<bool>,
    #[zvariant(rename = "type")]
    pub item_type: Option<String>,
    #[zvariant(rename = "toggle-type")]
    pub toggle_type: Option<String>,
    #[zvariant(rename = "toggle-state")]
    pub toggle_state: Option<i32>,
    pub shortcut: Option<Vec<Vec<String>>>,
}

/// D-Bus proxy for com.canonical.dbusmenu interface.
#[proxy(interface = "com.canonical.dbusmenu")]
pub trait DBusMenu {
    fn get_layout(&self, parent_id: i32, recursion_depth: i32, property_names: &[&str]) -> zbus::Result<(u32, Layout)>;

    fn event(&self, id: i32, event_id: &str, data: &OwnedValue, timestamp: u32) -> zbus::Result<()>;

    fn about_to_show(&self, id: i32) -> zbus::Result<bool>;

    #[zbus(signal)]
    fn layout_updated(&self, revision: u32, parent: i32) -> zbus::Result<()>;
}
