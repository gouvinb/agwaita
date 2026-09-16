//! Bluetooth pairing dialog component.

use crate::{
    model::{
        BluetoothStore,
        PairingRequest,
        PairingRequestKind,
        PairingResponse,
    },
    service::BluetoothServiceAdapter,
};
use relm4::{
    ComponentParts,
    ComponentSender,
    SimpleComponent,
    adw::{
        self,
        prelude::*,
    },
    gtk,
};
use std::sync::Arc;

#[derive(Debug)]
pub enum PairingDialogInput {
    Show(PairingRequest),
    Close,
    ConfirmClicked,
    CancelClicked,
    DialogClosed,
}

pub struct PairingDialogConfig {
    pub service_adapter: Arc<BluetoothServiceAdapter>,
    pub parent_window: adw::Window,
}

pub struct PairingDialog {
    service_adapter: Arc<BluetoothServiceAdapter>,
    parent_window: adw::Window,
    current_request: Option<PairingRequest>,
    dialog: adw::Dialog,
    title_label: gtk::Label,
    instructions_label: gtk::Label,
    code_label: gtk::Label,
    pin_entry: gtk::Entry,
    passkey_entry: gtk::Entry,
    confirm_button: gtk::Button,
    cancel_button: gtk::Button,
    is_responding: bool,
}

#[relm4::component(pub)]
impl SimpleComponent for PairingDialog {
    type Input = PairingDialogInput;
    type Output = ();
    type Init = PairingDialogConfig;

    view! {
        #[root]
        #[name = "dialog"]
        adw::Dialog {
            set_title: "Bluetooth Pairing",
            set_content_width: 380,

            #[wrap(Some)]
            set_child = &adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {
                    set_show_end_title_buttons: true,
                },

                #[wrap(Some)]
                set_content = &gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 16,
                    set_margin_start: 24,
                    set_margin_end: 24,
                    set_margin_top: 12,
                    set_margin_bottom: 24,
                    set_halign: gtk::Align::Fill,

                    #[name = "title_label"]
                    gtk::Label {
                        set_label: "Bluetooth Pairing",
                        add_css_class: "title-2",
                        set_wrap: true,
                        set_justify: gtk::Justification::Center,
                    },

                    #[name = "instructions_label"]
                    gtk::Label {
                        set_label: "",
                        add_css_class: "body",
                        set_wrap: true,
                        set_justify: gtk::Justification::Center,
                    },

                    #[name = "code_label"]
                    gtk::Label {
                        set_label: "",
                        add_css_class: "title-1",
                        add_css_class: "numeric",
                        set_visible: false,
                        set_selectable: true,
                    },

                    #[name = "pin_entry"]
                    gtk::Entry {
                        set_placeholder_text: Some("PIN Code"),
                        set_visible: false,
                        set_halign: gtk::Align::Fill,
                        connect_activate[sender] => move |_| {
                            sender.input(PairingDialogInput::ConfirmClicked);
                        },
                    },

                    #[name = "passkey_entry"]
                    gtk::Entry {
                        set_placeholder_text: Some("Passkey (000000 - 999999)"),
                        set_visible: false,
                        set_input_purpose: gtk::InputPurpose::Digits,
                        set_halign: gtk::Align::Fill,
                        connect_activate[sender] => move |_| {
                            sender.input(PairingDialogInput::ConfirmClicked);
                        },
                    },

                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 12,
                        set_halign: gtk::Align::Center,
                        set_margin_top: 8,

                        #[name = "cancel_button"]
                        gtk::Button {
                            set_label: "Cancel",
                            connect_clicked[sender] => move |_| {
                                sender.input(PairingDialogInput::CancelClicked);
                            },
                        },

                        #[name = "confirm_button"]
                        gtk::Button {
                            set_label: "Confirm",
                            add_css_class: "suggested-action",
                            set_visible: false,
                            connect_clicked[sender] => move |_| {
                                sender.input(PairingDialogInput::ConfirmClicked);
                            },
                        },
                    },
                },
            },
        }
    }

    fn init(config: Self::Init, root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let widgets = view_output!();

        widgets.dialog.connect_closed(gtk::glib::clone!(
            #[strong]
            sender,
            move |_| {
                sender.input(PairingDialogInput::DialogClosed);
            }
        ));

        let model = Self {
            service_adapter: config.service_adapter,
            parent_window: config.parent_window,
            current_request: None,
            dialog: widgets.dialog.clone(),
            title_label: widgets.title_label.clone(),
            instructions_label: widgets.instructions_label.clone(),
            code_label: widgets.code_label.clone(),
            pin_entry: widgets.pin_entry.clone(),
            passkey_entry: widgets.passkey_entry.clone(),
            confirm_button: widgets.confirm_button.clone(),
            cancel_button: widgets.cancel_button.clone(),
            is_responding: false,
        };

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            PairingDialogInput::Show(request) => {
                self.is_responding = false;
                self.current_request = Some(request.clone());
                self.render_request(&request);
                self.dialog.present(Some(&self.parent_window));
            },
            PairingDialogInput::Close => {
                self.current_request = None;
                self.is_responding = false;
                self.dialog.close();
            },
            PairingDialogInput::ConfirmClicked => {
                if let Some(req) = self.current_request.take() {
                    self.is_responding = true;
                    let response = match req.kind {
                        PairingRequestKind::RequestConfirmation { .. }
                        | PairingRequestKind::RequestAuthorization
                        | PairingRequestKind::RequestServiceAuthorization { .. } => Some(PairingResponse::Confirm),
                        PairingRequestKind::RequestPinCode => {
                            let pin = self.pin_entry.text().to_string();
                            Some(PairingResponse::PinCode(pin))
                        },
                        PairingRequestKind::RequestPasskey => {
                            let passkey_text = self.passkey_entry.text();
                            let passkey = passkey_text.parse::<u32>().unwrap_or(0);
                            Some(PairingResponse::Passkey(passkey))
                        },
                        PairingRequestKind::DisplayPasskey { .. } | PairingRequestKind::DisplayPinCode { .. } => {
                            self.service_adapter.store().clear_pending_pairing_request();
                            None
                        },
                    };

                    if let Some(resp) = response {
                        self.service_adapter.respond_pairing(resp);
                    }
                }
                self.dialog.close();
            },
            PairingDialogInput::CancelClicked => {
                if let Some(req) = self.current_request.take() {
                    self.is_responding = true;
                    match req.kind {
                        PairingRequestKind::DisplayPasskey { .. } | PairingRequestKind::DisplayPinCode { .. } => {
                            self.service_adapter.cancel_pairing(&req.device_address);
                        },
                        _ => {
                            self.service_adapter
                                .respond_pairing(PairingResponse::Reject);
                        },
                    }
                }
                self.dialog.close();
            },
            PairingDialogInput::DialogClosed => {
                // Si la boîte de dialogue est fermée par l'utilisateur (Escape, bouton fermer de la barre de titre, etc.)
                // sans clic explicite sur Confirmer/Annuler, on annule la requête en cours.
                if !self.is_responding
                    && let Some(req) = self.current_request.take()
                {
                    match req.kind {
                        PairingRequestKind::DisplayPasskey { .. } | PairingRequestKind::DisplayPinCode { .. } => {
                            self.service_adapter.cancel_pairing(&req.device_address);
                        },
                        _ => {
                            self.service_adapter
                                .respond_pairing(PairingResponse::Reject);
                        },
                    }
                }
            },
        }
    }
}

impl PairingDialog {
    fn render_request(&mut self, request: &PairingRequest) {
        let store = self.service_adapter.store();
        let device_name = Self::resolve_device_name(&store, &request.device_address);

        self.title_label
            .set_label(&format!("Pair with {}", device_name));

        // Reset visibility of dynamic parts
        self.code_label.set_visible(false);
        self.pin_entry.set_visible(false);
        self.passkey_entry.set_visible(false);
        self.confirm_button.set_visible(false);
        self.cancel_button.set_visible(true);
        self.cancel_button.set_label("Cancel");

        match &request.kind {
            PairingRequestKind::DisplayPasskey { passkey, .. } => {
                self.instructions_label
                    .set_label("Type this passkey on the device followed by Enter:");
                self.code_label.set_label(&format!("{:06}", passkey));
                self.code_label.set_visible(true);
                self.confirm_button.set_label("OK");
                self.confirm_button.set_visible(true);
            },
            PairingRequestKind::DisplayPinCode { pincode } => {
                self.instructions_label
                    .set_label("Enter this PIN code on the device:");
                self.code_label.set_label(pincode);
                self.code_label.set_visible(true);
                self.confirm_button.set_label("OK");
                self.confirm_button.set_visible(true);
            },
            PairingRequestKind::RequestConfirmation { passkey } => {
                self.instructions_label
                    .set_label("Confirm that the passkey matches the one displayed on the device:");
                self.code_label.set_label(&format!("{:06}", passkey));
                self.code_label.set_visible(true);
                self.confirm_button.set_label("Confirm");
                self.confirm_button.set_visible(true);
            },
            PairingRequestKind::RequestAuthorization => {
                self.instructions_label
                    .set_label("Authorize connection from this device?");
                self.confirm_button.set_label("Authorize");
                self.confirm_button.set_visible(true);
            },
            PairingRequestKind::RequestServiceAuthorization { service } => {
                self.instructions_label
                    .set_label(&format!("Authorize connection to service '{}'?", service));
                self.confirm_button.set_label("Authorize");
                self.confirm_button.set_visible(true);
            },
            PairingRequestKind::RequestPinCode => {
                self.instructions_label
                    .set_label("Enter the PIN code provided by the device:");
                self.pin_entry.set_text("");
                self.pin_entry.set_visible(true);
                self.pin_entry.grab_focus();
                self.confirm_button.set_label("Confirm");
                self.confirm_button.set_visible(true);
            },
            PairingRequestKind::RequestPasskey => {
                self.instructions_label
                    .set_label("Enter the passkey (000000 - 999999):");
                self.passkey_entry.set_text("");
                self.passkey_entry.set_visible(true);
                self.passkey_entry.grab_focus();
                self.confirm_button.set_label("Confirm");
                self.confirm_button.set_visible(true);
            },
        }
    }

    fn resolve_device_name(store: &BluetoothStore, address: &str) -> String {
        store
            .get_device(address)
            .map(|d| d.alias)
            .unwrap_or_else(|| address.to_string())
    }
}
