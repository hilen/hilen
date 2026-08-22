use std::net::{IpAddr, SocketAddr};

use anyhow::{Result, anyhow};
use hilen::{
    dispatch::{log_spawn, on_main},
    inspect::protocol::{
        AppCommand, Client, InspectorCommand, SERVICE_TYPE, UIRequest, UIResponse, ui::ViewRepr,
    },
    refs::Weak,
    ui::{
        AlertErr,
        Anchor::{Right, Top},
        Button, DropDown, Setup, UIEvent, UIManager, ViewData, async_link_button, view,
    },
};
use log::error;
use mdns_sd::{ScopedIp, ServiceDaemon, ServiceEvent};

use crate::ui::{
    common::ValueView,
    inspect::{SHRINK_SCALE, UIRepresentView, ViewInspectorView},
};

pub static VIEW_SELECTED: UIEvent<Weak<ViewRepr>> = UIEvent::const_new();

#[view]
pub struct MainScreen {
    apps:           Vec<(String, SocketAddr)>,
    current_client: Option<Client>,
    selected_id:    Option<String>,

    #[init]
    clients: DropDown<String>,

    play_sound:   Button,
    get_ui:       Button,
    ui_scale:     ValueView,
    shrink_scale: ValueView,

    ui_represent: UIRepresentView,

    inspect: ViewInspectorView,
}

impl Setup for MainScreen {
    fn setup(self: Weak<Self>) {
        self.clients.place().tl(10).size(200, 50);
        self.clients.on_changed(move |app_id| {
            self.connect_to(app_id);
        });

        self.play_sound.set_text("Play Sound").place().size(280, 50).tr(10);
        async_link_button!(self.play_sound, play_sound_tapped);

        self.get_ui.set_text("Get UI");
        self.get_ui.place().below(self.play_sound, 10);
        async_link_button!(self.get_ui, get_ui_tapped);

        self.ui_scale
            .set_title("UI scale")
            .place()
            .anchor(Top, self.get_ui, 10)
            .same_width(self.get_ui)
            .same_x(self.get_ui)
            .h(60);

        self.ui_scale.on_change.val_async(move |val| async move {
            self.scale_changed(val).await.alert_err();
        });

        self.shrink_scale.set_title("Shrink scale").place().below(self.ui_scale, 10);
        self.shrink_scale.on_change.val(move |val| {
            *SHRINK_SCALE.lock() = val;
            self.ui_represent.reload();
        });

        self.ui_represent
            .place()
            .l(20)
            .anchor(Top, self.clients, 20)
            .anchor(Right, self.play_sound, 20)
            .b(20);

        VIEW_SELECTED.val(self, move |view| {
            self.view_selected(view);
        });

        self.inspect
            .place()
            .same_x(self.play_sound)
            .same_width(self.play_sound)
            .anchor(Top, self.shrink_scale, 10)
            .b(10);

        self.inspect.placer_view.rule_changed.val(self, move |request| {
            log_spawn(async move {
                let response = self.client()?.send(request.into()).await?;
                self.process_command(response);
                Ok(())
            });
        });

        log_spawn(self.discover());
    }
}

impl MainScreen {
    async fn discover(self: Weak<Self>) -> Result<()> {
        let mdns = ServiceDaemon::new()?;
        let events = mdns.browse(SERVICE_TYPE)?;

        while let Ok(event) = events.recv_async().await {
            match event {
                ServiceEvent::ServiceResolved(service) => {
                    let Some(app_id) = service.txt_properties.get_property_val_str("app_id") else {
                        continue;
                    };

                    if app_id == UIManager::app_instance_id() {
                        continue;
                    }

                    let ip = service
                        .addresses
                        .iter()
                        .map(ScopedIp::to_ip_addr)
                        .find(IpAddr::is_ipv4)
                        .or_else(|| service.addresses.iter().next().map(ScopedIp::to_ip_addr));

                    let Some(ip) = ip else {
                        continue;
                    };

                    let app_id = app_id.to_string();
                    let addr = SocketAddr::new(ip, service.port);

                    on_main(move || self.app_discovered(app_id, addr));
                }
                ServiceEvent::ServiceRemoved(_, fullname) => {
                    on_main(move || self.app_removed(&fullname));
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn app_discovered(mut self: Weak<Self>, app_id: String, addr: SocketAddr) {
        if let Some(app) = self.apps.iter_mut().find(|(id, _)| *id == app_id) {
            app.1 = addr;
            return;
        }

        self.apps.push((app_id.clone(), addr));
        self.reload_clients();

        if self.current_client.is_none() {
            self.connect_to(app_id);
        }
    }

    fn app_removed(mut self: Weak<Self>, fullname: &str) {
        let app_id = fullname.strip_suffix(&format!(".{SERVICE_TYPE}")).unwrap_or(fullname);
        self.apps.retain(|(id, _)| id != app_id);
        self.reload_clients();
    }

    fn reload_clients(mut self: Weak<Self>) {
        let ids = self.apps.iter().map(|(id, _)| id.clone()).collect();
        self.clients.set_values(ids);
    }

    fn connect_to(self: Weak<Self>, app_id: String) {
        let Some((_, addr)) = self.apps.iter().find(|(id, _)| *id == app_id) else {
            return;
        };

        let addr = *addr;

        log_spawn(async move {
            let client = Client::connect(addr).await?;
            let response = client.send(UIRequest::GetUI.into()).await?;

            on_main(move || {
                self.set_client(client);
                self.process_command(response);
            });

            Ok(())
        });
    }

    fn set_client(mut self: Weak<Self>, client: Client) {
        self.current_client = Some(client);
    }

    fn view_selected(mut self: Weak<Self>, view: Weak<ViewRepr>) {
        self.selected_id = Some(view.id.clone());
        self.inspect.set_view(view);
    }

    fn restore_selection(self: Weak<Self>) {
        let Some(id) = self.selected_id.clone() else {
            return;
        };

        let Some(view) = find_repr(self.ui_represent.repr(), &id) else {
            return;
        };

        self.inspect.set_view(view);
    }

    fn client(&self) -> Result<&Client> {
        self.current_client.as_ref().ok_or_else(|| anyhow!("No client"))
    }

    async fn play_sound_tapped(self: Weak<Self>) -> Result<()> {
        self.client()?.send(InspectorCommand::PlaySound).await?;
        Ok(())
    }

    async fn get_ui_tapped(self: Weak<Self>) -> Result<()> {
        let response = self.client()?.send(UIRequest::GetUI.into()).await?;
        self.process_command(response);
        Ok(())
    }

    async fn scale_changed(self: Weak<Self>, scale: f32) -> Result<()> {
        let response = self.client()?.send(UIRequest::SetScale(scale).into()).await?;
        self.process_command(response);
        Ok(())
    }

    fn process_command(self: Weak<Self>, command: AppCommand) {
        match command {
            AppCommand::UI(UIResponse::SendUI { scale, root }) => on_main(move || {
                self.ui_represent.set_root(scale, root);
                self.restore_selection();
            }),
            AppCommand::Error(err) => error!("App returned an error: {err}"),
            AppCommand::Ok
            | AppCommand::Screenshot { .. }
            | AppCommand::Edits(_)
            | AppCommand::BuildTime(_)
            | AppCommand::StartTime(_)
            | AppCommand::TestResults { .. } => {}
        }
    }
}

fn find_repr(view: Weak<ViewRepr>, id: &str) -> Option<Weak<ViewRepr>> {
    if view.id == id {
        return Some(view);
    }

    view.subviews.iter().find_map(|sub| find_repr(sub.weak(), id))
}
