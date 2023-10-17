mod app;
mod components;
mod managers;
mod mock_components;

use rs_dapi_client::{AddressList, DapiClient, RequestSettings};
use tuirealm::{application::PollStrategy, AttrValue, Attribute, Update};

use app::{ComponentId, Model};

fn main() {
    // Setup DAPI client
    let mut address_list = AddressList::new();
    address_list.add_uri(rs_dapi_client::Uri::from_static(
        "https://54.213.204.85:1443",
    ));
    let mut dapi_client = DapiClient::new(address_list, RequestSettings::default());

    // Setup model
    let mut model = Model::new(&mut dapi_client);

    // Enter alternate screen
    let _ = model.terminal.enter_alternate_screen();
    let _ = model.terminal.enable_raw_mode();
    // Main loop
    while !model.quit {
        // Tick
        match model.app.tick(PollStrategy::Once) {
            Err(err) => {
                assert!(model
                    .app
                    .attr(
                        &ComponentId::Status,
                        Attribute::Text,
                        AttrValue::String(format!("Application error: {}", err)),
                    )
                    .is_ok());
            }
            Ok(messages) if messages.len() > 0 => {
                for msg in messages.into_iter() {
                    let mut msg = Some(msg);
                    while msg.is_some() {
                        msg = model.update(msg);
                    }
                }
            }
            _ => {}
        }
        // Redraw
        if model.redraw {
            model.view();
            model.redraw = false;
        }
    }
    // Terminate terminal
    let _ = model.terminal.leave_alternate_screen();
    let _ = model.terminal.disable_raw_mode();
    let _ = model.terminal.clear_screen();
}
