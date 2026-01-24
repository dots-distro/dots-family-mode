use gtk4::prelude::*;
use relm4::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApprovalRequest {
    pub id: String,
    pub profile_id: String,
    pub profile_name: String,
    pub request_type: String,
    pub details: String,
    pub created_at: String,
    pub status: String,
}

#[derive(Debug)]
pub enum ApprovalRequestCardMsg {
    Select,
}

#[relm4::factory(pub)]
impl FactoryComponent for ApprovalRequestCard {
    type Init = ApprovalRequest;
    type Input = ApprovalRequestCardMsg;
    type Output = String; // request_id
    type CommandOutput = ();
    type ParentWidget = gtk4::Box;

    view! {
        gtk4::Frame {
            add_css_class: "card",

            gtk4::Box {
                set_orientation: gtk4::Orientation::Vertical,
                set_spacing: 8,
                set_margin_top: 12,
                set_margin_bottom: 12,
                set_margin_start: 12,
                set_margin_end: 12,

                // Header with profile and type
                gtk4::Box {
                    set_orientation: gtk4::Orientation::Horizontal,
                    set_spacing: 12,

                    gtk4::Label {
                        #[watch]
                        set_label: &self.request.profile_name,
                        add_css_class: "heading",
                        set_halign: gtk4::Align::Start,
                        set_hexpand: true,
                    },

                    gtk4::Label {
                        #[watch]
                        set_label: &self.request.request_type,
                        add_css_class: "dim-label",
                    }
                },

                // Details
                gtk4::Label {
                    #[watch]
                    set_label: &self.request.details,
                    set_halign: gtk4::Align::Start,
                    set_wrap: true,
                    set_max_width_chars: 60,
                },

                // Timestamp
                gtk4::Label {
                    #[watch]
                    set_label: &format!("Requested: {}", self.request.created_at),
                    add_css_class: "caption",
                    add_css_class: "dim-label",
                    set_halign: gtk4::Align::Start,
                },

                // Select button
                gtk4::Button {
                    set_label: "Review",
                    set_halign: gtk4::Align::End,
                    connect_clicked => ApprovalRequestCardMsg::Select,
                }
            }
        }
    }

    fn init_model(init: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        Self { request: init }
    }

    fn update(&mut self, msg: Self::Input, sender: FactorySender<Self>) {
        match msg {
            ApprovalRequestCardMsg::Select => {
                let _ = sender.output(self.request.id.clone());
            }
        }
    }
}

pub struct ApprovalRequestCard {
    request: ApprovalRequest,
}
