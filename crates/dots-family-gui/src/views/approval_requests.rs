use dots_family_common::types::Profile;
use gtk4::prelude::*;
use relm4::{factory::FactoryVecDeque, prelude::*};

use crate::{
    components::approval_request_card::{ApprovalRequest, ApprovalRequestCard},
    daemon_client::DaemonClient,
};

pub struct ApprovalRequests {
    profile: Profile,
    daemon_client: Option<DaemonClient>,
    request_cards: FactoryVecDeque<ApprovalRequestCard>,
    selected_request: Option<String>,
    response_message: String,
}

#[derive(Debug)]
pub enum ApprovalRequestsMsg {
    UpdateProfile(Profile),
    RefreshRequests,
    UpdateRequests(Vec<ApprovalRequest>),
    SelectRequest(String),
    ApproveSelected,
    DenySelected,
    UpdateResponseMessage(String),
    ShowMessage(String),
}

#[relm4::component(pub)]
impl SimpleComponent for ApprovalRequests {
    type Init = Profile;
    type Input = ApprovalRequestsMsg;
    type Output = String;

    view! {
        gtk4::ScrolledWindow {
            set_policy: (gtk4::PolicyType::Never, gtk4::PolicyType::Automatic),

            #[wrap(Some)]
            set_child = &gtk4::Box {
                set_orientation: gtk4::Orientation::Vertical,
                set_spacing: 20,
                set_margin_all: 20,

                gtk4::Box {
                    set_orientation: gtk4::Orientation::Horizontal,
                    set_spacing: 12,

                    gtk4::Label {
                        set_label: "Approval Requests",
                        add_css_class: "title-1",
                        set_halign: gtk4::Align::Start,
                        set_hexpand: true,
                    },

                    gtk4::Button {
                        set_icon_name: "view-refresh-symbolic",
                        set_tooltip_text: Some("Refresh"),
                        connect_clicked => ApprovalRequestsMsg::RefreshRequests,
                    }
                },

                gtk4::Box {
                    set_orientation: gtk4::Orientation::Vertical,
                    set_spacing: 12,

                    #[watch]
                    set_visible: model.request_cards.is_empty(),

                    gtk4::Box {
                        set_orientation: gtk4::Orientation::Vertical,
                        set_spacing: 12,
                        set_valign: gtk4::Align::Center,
                        set_vexpand: true,
                        set_margin_all: 40,

                        gtk4::Image {
                            set_icon_name: Some("emblem-ok-symbolic"),
                            set_pixel_size: 64,
                            add_css_class: "success",
                        },

                        gtk4::Label {
                            set_label: "No Pending Requests",
                            add_css_class: "title-2",
                        },

                        gtk4::Label {
                            set_label: "All approval requests have been handled",
                            add_css_class: "dim-label",
                        }
                    }
                },

                gtk4::Box {
                    set_orientation: gtk4::Orientation::Vertical,
                    set_spacing: 12,

                    #[watch]
                    set_visible: !model.request_cards.is_empty(),

                    gtk4::Label {
                        #[watch]
                        set_label: &format!("{} pending request{}",
                            model.request_cards.len(),
                            if model.request_cards.len() == 1 { "" } else { "s" }
                        ),
                        add_css_class: "title-2",
                        set_halign: gtk4::Align::Start,
                    },

                    #[local_ref]
                    requests_box -> gtk4::Box {
                        set_orientation: gtk4::Orientation::Vertical,
                        set_spacing: 12,
                    }
                },

                // Action buttons section
                gtk4::Box {
                    set_orientation: gtk4::Orientation::Vertical,
                    set_spacing: 12,
                    set_margin_top: 20,

                    #[watch]
                    set_visible: model.selected_request.is_some(),

                    gtk4::Separator {},

                    gtk4::Label {
                        set_label: "Response Message (optional)",
                        set_halign: gtk4::Align::Start,
                        add_css_class: "heading",
                    },

                    gtk4::Entry {
                        set_placeholder_text: Some("Enter a message for the child (optional)"),
                        #[watch]
                        set_text: &model.response_message,
                        connect_changed[sender] => move |entry| {
                            sender.input(ApprovalRequestsMsg::UpdateResponseMessage(
                                entry.text().to_string()
                            ));
                        }
                    },

                    gtk4::Box {
                        set_orientation: gtk4::Orientation::Horizontal,
                        set_spacing: 12,
                        set_halign: gtk4::Align::End,

                        gtk4::Button {
                            set_label: "Deny",
                            add_css_class: "destructive-action",
                            set_icon_name: "window-close-symbolic",
                            connect_clicked => ApprovalRequestsMsg::DenySelected,
                        },

                        gtk4::Button {
                            set_label: "Approve",
                            add_css_class: "suggested-action",
                            set_icon_name: "emblem-ok-symbolic",
                            connect_clicked => ApprovalRequestsMsg::ApproveSelected,
                        }
                    }
                }
            }
        }
    }

    fn init(
        profile: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        // Create the container for request cards
        let requests_box =
            gtk4::Box::builder().orientation(gtk4::Orientation::Vertical).spacing(12).build();

        // Initialize the Factory
        let request_cards = FactoryVecDeque::builder()
            .launch(requests_box.clone())
            .forward(sender.input_sender(), |request_id| {
                ApprovalRequestsMsg::SelectRequest(request_id)
            });

        let model = ApprovalRequests {
            profile,
            daemon_client: None,
            request_cards,
            selected_request: None,
            response_message: String::new(),
        };

        let widgets = view_output!();

        // Initialize daemon client asynchronously
        let sender_clone = sender.clone();
        relm4::spawn(async move {
            let client = DaemonClient::new().await;
            if client.connect().await.is_ok() {
                // Store client somehow - for now just trigger refresh
                sender_clone.input(ApprovalRequestsMsg::RefreshRequests);
            }
        });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            ApprovalRequestsMsg::UpdateProfile(profile) => {
                self.profile = profile;
            }
            ApprovalRequestsMsg::RefreshRequests => {
                if let Some(daemon_client) = &self.daemon_client {
                    let daemon_client = daemon_client.clone();
                    relm4::spawn(async move {
                        // TODO: Get token from authentication
                        // For now, this will fail gracefully
                        let token = ""; // Placeholder - needs parent auth

                        match daemon_client.list_pending_requests(token).await {
                            Ok(response_json) => {
                                match serde_json::from_str::<Vec<ApprovalRequest>>(&response_json) {
                                    Ok(requests) => {
                                        sender.input(ApprovalRequestsMsg::UpdateRequests(requests));
                                    }
                                    Err(e) => {
                                        eprintln!("Failed to parse approval requests: {}", e);
                                        sender.input(ApprovalRequestsMsg::UpdateRequests(vec![]));
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("Failed to fetch approval requests: {}", e);
                                sender.input(ApprovalRequestsMsg::UpdateRequests(vec![]));
                            }
                        }
                    });
                } else {
                    eprintln!("Daemon client not initialized");
                }
            }
            ApprovalRequestsMsg::UpdateRequests(requests) => {
                // Clear existing cards
                let mut guard = self.request_cards.guard();
                guard.clear();

                // Add new cards
                for request in requests {
                    guard.push_back(request);
                }
            }
            ApprovalRequestsMsg::SelectRequest(request_id) => {
                self.selected_request = Some(request_id);
                self.response_message.clear();
            }
            ApprovalRequestsMsg::ApproveSelected => {
                if let (Some(request_id), Some(daemon_client)) =
                    (&self.selected_request, &self.daemon_client)
                {
                    let request_id = request_id.clone();
                    let message = self.response_message.clone();
                    let daemon_client = daemon_client.clone();

                    relm4::spawn(async move {
                        let token = ""; // TODO: Get token from authentication

                        match daemon_client.approve_request(&request_id, &message, token).await {
                            Ok(_response) => {
                                sender.input(ApprovalRequestsMsg::ShowMessage(
                                    "Request approved successfully".to_string(),
                                ));
                                sender.input(ApprovalRequestsMsg::RefreshRequests);
                            }
                            Err(e) => {
                                sender.input(ApprovalRequestsMsg::ShowMessage(format!(
                                    "Failed to approve request: {}",
                                    e
                                )));
                            }
                        }
                    });

                    self.selected_request = None;
                    self.response_message.clear();
                }
            }
            ApprovalRequestsMsg::DenySelected => {
                if let (Some(request_id), Some(daemon_client)) =
                    (&self.selected_request, &self.daemon_client)
                {
                    let request_id = request_id.clone();
                    let message = self.response_message.clone();
                    let daemon_client = daemon_client.clone();

                    relm4::spawn(async move {
                        let token = ""; // TODO: Get token from authentication

                        match daemon_client.deny_request(&request_id, &message, token).await {
                            Ok(_response) => {
                                sender.input(ApprovalRequestsMsg::ShowMessage(
                                    "Request denied".to_string(),
                                ));
                                sender.input(ApprovalRequestsMsg::RefreshRequests);
                            }
                            Err(e) => {
                                sender.input(ApprovalRequestsMsg::ShowMessage(format!(
                                    "Failed to deny request: {}",
                                    e
                                )));
                            }
                        }
                    });

                    self.selected_request = None;
                    self.response_message.clear();
                }
            }
            ApprovalRequestsMsg::UpdateResponseMessage(message) => {
                self.response_message = message;
            }
            ApprovalRequestsMsg::ShowMessage(message) => {
                let _ = sender.output(message);
            }
        }
    }
}
