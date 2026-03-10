use catalyser::stdx::extension::str_extension::MultilineStr;
use chrono::{
    DateTime,
    Local,
};
use gtk4::prelude::*;
use relm4::{
    RelmWidgetExt,
    factory::FactoryComponent,
    gtk,
};

#[derive(Debug, Clone)]
pub struct EventData {
    pub title: String,
    pub description: Option<String>,
    pub start: DateTime<Local>,
    pub end: DateTime<Local>,
    pub color: String,
    pub is_all_day: bool,
}

pub struct EventItem {
    event: EventData,
}

#[relm4::factory(pub)]
impl FactoryComponent for EventItem {
    type Init = EventData;
    type Input = ();
    type Output = ();
    type CommandOutput = ();
    type ParentWidget = gtk::Box;

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,

            gtk::Box {
                inline_css: &format!("
                |background: {};
                |border-top-left-radius: 5px;
                |border-bottom-left-radius: 5px;
                ", self.event.color).trim_margin().as_str(),
                set_orientation: gtk::Orientation::Vertical,
                set_width_request: 4,
            },

            gtk::Box {
                inline_css: &format!("
                |background: alpha({}, 0.25);
                |border-top-right-radius: 5px;
                |border-bottom-right-radius: 5px;
                ", self.event.color).trim_margin().as_str(),
                set_hexpand: true,

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_margin_all: 4,
                    set_width_request: 320 - 16 -4,

                    gtk::Label {
                        add_css_class: "title-5",
                        inline_css: "
                        |font-weight: bold;
                        ".trim_margin().as_str(),
                        set_halign: gtk::Align::Start,
                        set_valign: gtk::Align::Fill,
                        set_ellipsize: gtk::pango::EllipsizeMode::End,

                        set_label: &self.event.title,
                    },
                    gtk::Label {
                        add_css_class: "caption",
                        set_halign: gtk::Align::Start,
                        set_valign: gtk::Align::Fill,
                        set_ellipsize: gtk::pango::EllipsizeMode::End,
                        set_margin_bottom : 4,
                        set_visible: !self.event.is_all_day,

                        set_label: &format!(
                            "{} - {}",
                            self.event.start.format("%H:%M"),
                            self.event.end.format("%H:%M")
                        ),
                    },
                    gtk::Label {
                        add_css_class: "caption",
                        set_visible: !self.event.description.clone().unwrap_or_default().is_empty(),
                        set_halign: gtk::Align::Start,
                        set_hexpand: true,
                        set_wrap: true,
                        set_wrap_mode: gtk::pango::WrapMode::WordChar,
                        set_natural_wrap_mode: gtk::NaturalWrapMode::Word,
                        set_max_width_chars: 1,
                        set_use_markup: true,


                        set_label: gtk::glib::markup_escape_text(self.event.description.as_deref().unwrap_or("")).as_str(),
                    },
                },
            },
        }
    }

    fn init_model(event: Self::Init, _index: &relm4::factory::DynamicIndex, _sender: relm4::FactorySender<Self>) -> Self {
        EventItem { event }
    }
}
