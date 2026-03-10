use crate::component::quick_settings_menu::icons::brightness::icon::{
    BrightnessIcon,
    BrightnessIconInput,
};
use agw_service::brightness::BrightnessService;
use gtk4::prelude::*;
use relm4::{
    Component,
    ComponentController,
    ComponentParts,
    ComponentSender,
    RelmWidgetExt,
    gtk,
};
use std::{
    cell::Cell,
    sync::Arc,
};

/// BrightnessSlider - Screen brightness control slider with icon
pub struct BrightnessSlider {
    brightness_icon: relm4::Controller<BrightnessIcon>,
    brightness_service: Arc<BrightnessService>,
    brightness: f64,
    is_dragging: Cell<bool>,
    is_programmatic_update: Cell<bool>,
}

#[derive(Debug)]
pub enum BrightnessSliderInput {
    UpdateLevel(f64),
    BrightnessChanged(f64),
}

#[relm4::component(pub)]
impl Component for BrightnessSlider {
    type Init = Arc<BrightnessService>;
    type Input = BrightnessSliderInput;
    type Output = ();
    type CommandOutput = ();

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,
            set_spacing: 8,
            set_hexpand: true,

            // Brightness icon wrapped in button for alignment
            gtk::Button {
                set_has_frame: false,
                set_sensitive: false,
                model.brightness_icon.widget().clone(),
            },

            // Brightness slider (0-100%)
            #[name = "brightness_scale"]
            gtk::Scale {
                set_hexpand: true,
                set_draw_value: false,
                set_range: (0.0, 1.0),
                set_increments: (0.01, 0.1),
                #[watch]
                set_value: model.brightness,
                connect_value_changed[sender] => move |scale| {
                    sender.input(BrightnessSliderInput::BrightnessChanged(scale.value()));
                },
            },

            // Percentage label
            gtk::Label {
                #[watch]
                set_label: &format!("{}%", (model.brightness * 100.0).round() as i32),
                set_width_chars: 4,
            },

            // Settings button to open pavucontrol
            gtk::ToggleButton {
                inline_css: "opacity: 0;",

                set_icon_name: "preferences-other-symbolic",
            },
        }
    }

    fn init(service: Self::Init, root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        // Get initial state from service
        let brightness = service.get_brightness();

        // Initialize brightness icon
        let brightness_icon = BrightnessIcon::builder()
            .launch(brightness)
            .forward(sender.input_sender(), |_| unreachable!());

        let model = BrightnessSlider {
            brightness_icon,
            brightness_service: service,
            brightness,
            is_dragging: Cell::new(false),
            is_programmatic_update: Cell::new(false),
        };

        let widgets = view_output!();

        // Add gesture controller to track drag start/end
        let gesture = gtk::GestureClick::new();
        gesture.set_button(1); // Left click only

        let is_dragging = model.is_dragging.clone();
        gesture.connect_pressed(move |_, _, _, _| {
            is_dragging.set(true);
        });

        let is_dragging = model.is_dragging.clone();
        gesture.connect_released(move |_, _, _, _| {
            is_dragging.set(false);
        });

        widgets.brightness_scale.add_controller(gesture);

        // Désactiver complètement le scroll sur le slider pour éviter les boucles infinies
        let scroll_controller = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::VERTICAL);
        scroll_controller.connect_scroll(|_, _, _| {
            gtk::glib::Propagation::Stop // Bloque tous les événements de scroll
        });
        widgets.brightness_scale.add_controller(scroll_controller);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, #[allow(unused_variables)] sender: ComponentSender<Self>, #[allow(unused_variables)] root: &Self::Root) {
        match message {
            BrightnessSliderInput::UpdateLevel(brightness) => {
                // Ignore ALL updates while user is dragging (avoid stuttering from inotify events)
                if self.is_dragging.get() {
                    return;
                }

                // Only update if the percentage value changed
                let old_percent = (self.brightness * 100.0).round() as i32;
                let new_percent = (brightness * 100.0).round() as i32;

                if old_percent != new_percent {
                    // Set flag to prevent connect_value_changed from triggering set_brightness
                    self.is_programmatic_update.set(true);
                    self.brightness = brightness;
                    // Flag will be cleared when connect_value_changed is called

                    // Update icon
                    self.brightness_icon
                        .emit(BrightnessIconInput::UpdateLevel(brightness));
                }
            },
            BrightnessSliderInput::BrightnessChanged(brightness) => {
                // Check if this is a programmatic update (from inotify)
                if self.is_programmatic_update.get() {
                    // Clear flag and ignore this event (avoid loop)
                    self.is_programmatic_update.set(false);
                    return;
                }

                // This is a user-initiated change (drag/click)
                let old_percent = (self.brightness * 100.0) as i32;
                let new_percent = (brightness * 100.0) as i32;

                self.brightness = brightness;
                self.brightness_icon
                    .emit(BrightnessIconInput::UpdateLevel(brightness));

                if old_percent != new_percent {
                    self.brightness_service.set_brightness(brightness);
                }
            },
        }
    }
}
