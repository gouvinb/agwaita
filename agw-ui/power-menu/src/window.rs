use crate::model::PowerMenuAction;
use catalyser::stdx::extension::str_extension::MultilineStr;
use gtk4::{
    gdk,
    glib,
    prelude::*,
};
use gtk4_layer_shell::{
    Edge,
    KeyboardMode,
    Layer,
    LayerShell,
};
use log::debug;
use relm4::{
    ComponentParts,
    ComponentSender,
    RelmWidgetExt,
    SimpleComponent,
    adw::{
        self,
        prelude::*,
    },
    gtk,
};

pub struct PowerMenuWindow {
    visible: bool,
    actions: Vec<PowerMenuAction>,
    window: gtk::Window,
    selection_model: gtk::SingleSelection,
}

#[derive(Debug, Clone)]
pub enum PowerMenuWindowInput {
    Toggle,
    Hide,
    ExecuteAction(usize),
    NavigateDown,
    NavigateUp,
    ActivateSelected,
}

#[derive(Debug, Clone, Default)]
pub struct PowerMenuWindowConfig;

#[relm4::component(pub)]
impl SimpleComponent for PowerMenuWindow {
    type Input = PowerMenuWindowInput;
    type Output = ();
    type Init = PowerMenuWindowConfig;

    view! {
        #[root]
        gtk::Window {
            set_namespace: Some("agwaita-power-menu"),
            set_layer: Layer::Overlay,
            set_anchor: (Edge::Top, true),
            set_anchor: (Edge::Right, true),
            set_anchor: (Edge::Left, true),
            set_anchor: (Edge::Bottom, true),
            set_keyboard_mode: KeyboardMode::Exclusive,
            inline_css: "background: alpha(var(--window-bg-color), 0.25);",
            set_visible: false,

            add_controller = gtk::EventControllerKey {
                connect_key_pressed[sender] => move |_, keyval, _, _| {
                    match keyval {
                        gdk::Key::Escape => {
                            sender.input(PowerMenuWindowInput::Hide);
                            glib::Propagation::Stop
                        }
                        gdk::Key::Down => {
                            sender.input(PowerMenuWindowInput::NavigateDown);
                            glib::Propagation::Stop
                        }
                        gdk::Key::Up => {
                            sender.input(PowerMenuWindowInput::NavigateUp);
                            glib::Propagation::Stop
                        }
                        gdk::Key::Return | gdk::Key::KP_Enter => {
                            sender.input(PowerMenuWindowInput::ActivateSelected);
                            glib::Propagation::Stop
                        }
                        _ => glib::Propagation::Proceed
                    }
                }
            },

            add_controller = gtk::GestureClick {
                connect_released[sender] => move |gesture, _, x, y| {
                    if let Some(widget) = gesture.widget() {
                        if let Some(window) = widget.downcast_ref::<gtk::Window>() {
                            if let Some(picked) = window.pick(x, y, gtk::PickFlags::DEFAULT) {
                                if picked.is::<gtk::Window>() {
                                    sender.input(PowerMenuWindowInput::Hide);
                                }
                            }
                        }
                    }
                }
            },

            gtk::Box {
                add_css_class: "card",
                inline_css: "
                |background-color: @window_bg_color;
                |padding: 15px;
                ".trim_margin().as_str(),
                set_halign: gtk::Align::Center,
                set_valign: gtk::Align::Center,
                set_orientation: gtk::Orientation::Vertical,

                #[local_ref]
                action_list -> gtk::ListView {
                    add_css_class: "card",
                    add_css_class: "frame",
                    set_can_focus: true,
                }
            }
        }
    }

    fn init(_config: Self::Init, root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        root.init_layer_shell();

        let actions = PowerMenuAction::ALL_ACTIONS.to_vec();

        let list_store = gtk::gio::ListStore::new::<glib::BoxedAnyObject>();
        for action in actions.iter() {
            list_store.append(&glib::BoxedAnyObject::new(action.clone()));
        }

        let selection_model = gtk::SingleSelection::new(Some(list_store.clone()));
        selection_model.set_autoselect(false);
        selection_model.set_can_unselect(true);

        let factory = gtk::SignalListItemFactory::new();

        let sender_for_setup = sender.clone();
        let selection_model_for_setup = selection_model.clone();
        factory.connect_setup(move |_, list_item| {
            let list_item_ref = list_item.downcast_ref::<gtk::ListItem>().unwrap();
            let row = adw::ActionRow::new();

            // Add click handler
            let gesture = gtk::GestureClick::new();
            gesture.set_button(gdk::BUTTON_PRIMARY);
            let sender_for_click = sender_for_setup.clone();
            let selection_model_clone = selection_model_for_setup.clone();
            let list_item_for_click = list_item_ref.clone();
            gesture.connect_released(move |_, _, _, _| {
                let position = list_item_for_click.position();
                selection_model_clone.set_selected(position);
                sender_for_click.input(PowerMenuWindowInput::ExecuteAction(position as usize));
            });
            row.add_controller(gesture);

            list_item_ref.set_child(Some(&row));
        });

        factory.connect_bind(move |_, list_item| {
            let list_item = list_item.downcast_ref::<gtk::ListItem>().unwrap();
            let row = list_item
                .child()
                .unwrap()
                .downcast::<adw::ActionRow>()
                .unwrap();

            if let Some(obj) = list_item.item() {
                let boxed = obj.downcast_ref::<glib::BoxedAnyObject>().unwrap();
                let action: PowerMenuAction = boxed.borrow::<PowerMenuAction>().clone();

                row.set_title(action.title);
                row.add_prefix(&gtk::Image::from_icon_name(action.icon_name));
            }
        });

        let action_list = gtk::ListView::new(Some(selection_model.clone()), Some(factory));

        let widgets = view_output!();

        let model = Self {
            visible: false,
            actions,
            window: root,
            selection_model,
        };

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            PowerMenuWindowInput::Toggle => {
                self.visible = !self.visible;
                self.window.set_visible(self.visible);

                if self.visible {
                    // Select first item when opening
                    self.selection_model.set_selected(0);
                }

                debug!("Power menu toggled: visible={}", self.visible);
            },
            PowerMenuWindowInput::Hide => {
                self.visible = false;
                self.window.set_visible(false);
                debug!("Power menu hidden");
            },
            PowerMenuWindowInput::ExecuteAction(idx) => {
                if let Some(action) = self.actions.get(idx) {
                    debug!("Executing power menu action: {}", action.title);
                    sender.input(PowerMenuWindowInput::Hide);

                    if let Err(e) = action.call() {
                        log::error!("Power menu action '{}' failed: {}", action.title, e);
                    }
                }
            },
            PowerMenuWindowInput::NavigateDown => {
                let current = self.selection_model.selected();
                let max = self.actions.len().saturating_sub(1) as u32;
                if current < max {
                    self.selection_model.set_selected(current + 1);
                }
            },
            PowerMenuWindowInput::NavigateUp => {
                let current = self.selection_model.selected();
                if current > 0 {
                    self.selection_model.set_selected(current - 1);
                }
            },
            PowerMenuWindowInput::ActivateSelected => {
                let idx = self.selection_model.selected() as usize;
                sender.input(PowerMenuWindowInput::ExecuteAction(idx));
            },
        }
    }
}
