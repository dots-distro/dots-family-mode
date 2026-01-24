use dots_family_common::types::Profile;
use futures::StreamExt;
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
    auth_token: Option<String>,
    parent_password: String,
    show_auth_dialog: bool,
    auth_failed: bool,
    error_message: Option<String>,
    show_loading: bool,
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
    ShowAuthDialog,
    ParentPasswordChanged(String),
    AttemptAuth,
    AuthenticationResult(bool, Option<String>),
    DaemonClientReady(DaemonClient),
    ApprovalRequestSignal(String, String), // request_id, request_type
    ShowError(String),
    DismissError,
}

#[relm4::component(pub)]
impl SimpleComponent for ApprovalRequests {
    type Init = Profile;
    type Input = ApprovalRequestsMsg;
    type Output = String;

    view! {
            gtk4::Box {
                set_orientation: gtk4::Orientation::Vertical,

                // Authentication Dialog Overlay
                gtk4::Box {
                    set_orientation: gtk4::Orientation::Vertical,
                    set_spacing: 20,
                    set_margin_all: 40,
                    set_valign: gtk4::Align::Center,
                    set_halign: gtk4::Align::Center,

                    #[watch]
                    set_visible: model.show_auth_dialog,

                    gtk4::Frame {
                        set_width_request: 400,
                        add_css_class: "card",

                        gtk4::Box {
                            set_orientation: gtk4::Orientation::Vertical,
                            set_spacing: 16,
                            set_margin_all: 24,

                            gtk4::Label {
                                set_label: "Parent Authentication Required",
                                add_css_class: "title-2",
                            },

                            gtk4::Label {
                                set_label: "Enter your parent password to view approval requests",
                                add_css_class: "dim-label",
                                set_wrap: true,
                            },

                            gtk4::PasswordEntry {
                                set_placeholder_text: Some("Parent password"),
                                #[watch]
                                set_text: &model.parent_password,
                                set_show_peek_icon: true,
                                connect_changed[sender] => move |entry| {
                                    sender.input(ApprovalRequestsMsg::ParentPasswordChanged(
                                        entry.text().to_string()
                                    ));
                                },
                                connect_activate => ApprovalRequestsMsg::AttemptAuth,
                            },

                            gtk4::Box {
                                #[watch]
                                set_visible: model.auth_failed,
                                set_orientation: gtk4::Orientation::Horizontal,
                                set_spacing: 8,

                                gtk4::Image {
                                    set_icon_name: Some("dialog-error-symbolic"),
                                    add_css_class: "error",
                                },

                                gtk4::Label {
                                    set_label: "Authentication failed. Please check your password.",
                                    add_css_class: "error",
                                    set_wrap: true,
                                }
                            },

                            gtk4::Button {
                                set_label: "Authenticate",
                                add_css_class: "suggested-action",
                                #[watch]
                                set_sensitive: !model.parent_password.is_empty(),
                                connect_clicked => ApprovalRequestsMsg::AttemptAuth,
                            }
                        }
                    }
                },

                // Error Dialog Overlay
                gtk4::Box {
                    set_orientation: gtk4::Orientation::Vertical,
                    set_spacing: 20,
                    set_margin_all: 40,
                    set_valign: gtk4::Align::Center,
                    set_halign: gtk4::Align::Center,

                    #[watch]
                    set_visible: model.error_message.is_some(),

                    gtk4::Frame {
                        set_width_request: 400,
                        add_css_class: "card",

                        gtk4::Box {
                            set_orientation: gtk4::Orientation::Vertical,
                            set_spacing: 16,
                            set_margin_all: 24,

                            gtk4::Box {
                                set_orientation: gtk4::Orientation::Horizontal,
                                set_spacing: 12,

                                gtk4::Image {
                                    set_icon_name: Some("dialog-error-symbolic"),
                                    set_pixel_size: 48,
                                    add_css_class: "error",
                                },

                                gtk4::Box {
                                    set_orientation: gtk4::Orientation::Vertical,
                                    set_spacing: 8,
                                    set_hexpand: true,

                                    gtk4::Label {
                                        set_label: "Connection Error",
                                        add_css_class: "title-2",
                                        set_halign: gtk4::Align::Start,
                                    },

                                    gtk4::Label {
                                        #[watch]
                                        set_label: &model.error_message.as_deref().unwrap_or("Unknown error"),
                                        add_css_class: "dim-label",
                                        set_wrap: true,
                                        set_halign: gtk4::Align::Start,
                                    }
                                }
                            },

                            gtk4::Box {
                                set_orientation: gtk4::Orientation::Horizontal,
                                set_spacing: 8,
                                set_halign: gtk4::Align::End,

                                gtk4::Button {
                                    set_label: "Retry",
                                    connect_clicked => ApprovalRequestsMsg::RefreshRequests,
                                },

                                gtk4::Button {
                                    set_label: "Dismiss",
                                    add_css_class: "suggested-action",
                                    connect_clicked => ApprovalRequestsMsg::DismissError,
                                }
                            }
                        }
                    }
                },

                // Main Content (only visible when authenticated)
                gtk4::ScrolledWindow {
                    set_policy: (gtk4::PolicyType::Never, gtk4::PolicyType::Automatic),
                    #[watch]
                    set_visible: !model.show_auth_dialog && model.auth_token.is_some(),

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
                            #[watch]
                            set_sensitive: !model.show_loading,
                            connect_clicked => ApprovalRequestsMsg::RefreshRequests,
                        },

                        gtk4::Spinner {
                            #[watch]
                            set_visible: model.show_loading,
                            #[watch]
                            set_spinning: model.show_loading,
                        }
                    },

                    // Loading overlay
                    gtk4::Box {
                        set_orientation: gtk4::Orientation::Vertical,
                        set_spacing: 12,
                        set_valign: gtk4::Align::Center,
                        set_vexpand: true,
                        set_margin_all: 40,

                        #[watch]
                        set_visible: model.show_loading && model.request_cards.is_empty(),

                        gtk4::Spinner {
                            set_size_request: (48, 48),
                            set_spinning: true,
                        },

                        gtk4::Label {
                            set_label: "Loading requests...",
                            add_css_class: "dim-label",
                        }
                    },

                    gtk4::Box {
                        set_orientation: gtk4::Orientation::Vertical,
                        set_spacing: 12,

                        #[watch]
                        set_visible: model.request_cards.is_empty() && !model.show_loading,

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
            auth_token: None,
            parent_password: String::new(),
            show_auth_dialog: true,
            auth_failed: false,
            error_message: None,
            show_loading: false,
        };

        let widgets = view_output!();

        // Initialize daemon client asynchronously
        relm4::spawn_local({
            let sender = sender.clone();
            async move {
                let client = DaemonClient::new().await;
                if client.connect().await.is_ok() {
                    sender.input(ApprovalRequestsMsg::DaemonClientReady(client));
                } else {
                    eprintln!("Failed to connect to daemon");
                }
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
                if let (Some(daemon_client), Some(token)) = (&self.daemon_client, &self.auth_token)
                {
                    self.show_loading = true;
                    self.error_message = None;
                    let daemon_client = daemon_client.clone();
                    let token = token.clone();
                    relm4::spawn(async move {
                        match daemon_client.list_pending_requests(&token).await {
                            Ok(response_json) => {
                                match serde_json::from_str::<Vec<ApprovalRequest>>(&response_json) {
                                    Ok(requests) => {
                                        sender.input(ApprovalRequestsMsg::UpdateRequests(requests));
                                    }
                                    Err(e) => {
                                        sender.input(ApprovalRequestsMsg::ShowError(format!(
                                            "Failed to parse server response: {}",
                                            e
                                        )));
                                        sender.input(ApprovalRequestsMsg::UpdateRequests(vec![]));
                                    }
                                }
                            }
                            Err(e) => {
                                sender.input(ApprovalRequestsMsg::ShowError(format!(
                                    "Failed to connect to daemon: {}. Is it running?",
                                    e
                                )));
                                sender.input(ApprovalRequestsMsg::UpdateRequests(vec![]));
                            }
                        }
                    });
                } else {
                    self.error_message = Some(
                        "Not authenticated or daemon not connected. Please try again.".to_string(),
                    );
                }
            }
            ApprovalRequestsMsg::UpdateRequests(requests) => {
                self.show_loading = false;
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
                if let (Some(request_id), Some(daemon_client), Some(token)) =
                    (&self.selected_request, &self.daemon_client, &self.auth_token)
                {
                    let request_id = request_id.clone();
                    let message = self.response_message.clone();
                    let daemon_client = daemon_client.clone();
                    let token = token.clone();

                    relm4::spawn(async move {
                        match daemon_client.approve_request(&request_id, &message, &token).await {
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
                if let (Some(request_id), Some(daemon_client), Some(token)) =
                    (&self.selected_request, &self.daemon_client, &self.auth_token)
                {
                    let request_id = request_id.clone();
                    let message = self.response_message.clone();
                    let daemon_client = daemon_client.clone();
                    let token = token.clone();

                    relm4::spawn(async move {
                        match daemon_client.deny_request(&request_id, &message, &token).await {
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
            ApprovalRequestsMsg::ShowAuthDialog => {
                self.show_auth_dialog = true;
                self.auth_failed = false;
                self.parent_password.clear();
            }
            ApprovalRequestsMsg::ParentPasswordChanged(password) => {
                self.parent_password = password;
                self.auth_failed = false;
            }
            ApprovalRequestsMsg::AttemptAuth => {
                if let Some(daemon_client) = &self.daemon_client {
                    let daemon_client = daemon_client.clone();
                    let password = self.parent_password.clone();

                    relm4::spawn(async move {
                        match daemon_client.authenticate_parent(&password).await {
                            Ok(token) => {
                                // Check if authentication actually succeeded
                                if token.starts_with("error:") {
                                    sender.input(ApprovalRequestsMsg::AuthenticationResult(
                                        false, None,
                                    ));
                                } else {
                                    sender.input(ApprovalRequestsMsg::AuthenticationResult(
                                        true,
                                        Some(token),
                                    ));
                                }
                            }
                            Err(_) => {
                                sender
                                    .input(ApprovalRequestsMsg::AuthenticationResult(false, None));
                            }
                        }
                    });
                }
            }
            ApprovalRequestsMsg::AuthenticationResult(success, token) => {
                if success {
                    self.auth_token = token;
                    self.show_auth_dialog = false;
                    self.auth_failed = false;
                    self.parent_password.clear();
                    // Now fetch the requests
                    sender.input(ApprovalRequestsMsg::RefreshRequests);

                    // Subscribe to approval request signals for real-time updates
                    if let Some(daemon_client) = &self.daemon_client {
                        let daemon_client = daemon_client.clone();
                        let sender_clone = sender.clone();
                        relm4::spawn(async move {
                            let result = daemon_client
                                .subscribe_approval_requests(move |request_id, request_type| {
                                    sender_clone.input(ApprovalRequestsMsg::ApprovalRequestSignal(
                                        request_id,
                                        request_type,
                                    ));
                                })
                                .await;

                            if let Err(e) = result {
                                eprintln!("Failed to subscribe to approval signals: {}", e);
                            }
                        });
                    }
                } else {
                    self.auth_failed = true;
                    self.parent_password.clear();
                }
            }
            ApprovalRequestsMsg::DaemonClientReady(client) => {
                self.daemon_client = Some(client);
            }
            ApprovalRequestsMsg::ApprovalRequestSignal(_request_id, _request_type) => {
                // New approval request detected - refresh the list
                sender.input(ApprovalRequestsMsg::RefreshRequests);
            }
            ApprovalRequestsMsg::ShowError(error) => {
                self.error_message = Some(error);
                self.show_loading = false;
            }
            ApprovalRequestsMsg::DismissError => {
                self.error_message = None;
            }
        }
    }
}
