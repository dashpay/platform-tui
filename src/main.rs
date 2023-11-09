mod app;
mod components;
mod managers;
mod mock_components;

use app::{ComponentId, Model};
use dash_platform_sdk::{Sdk, SdkBuilder};
use dpp::version::PlatformVersion;
use rs_dapi_client::AddressList;
use tuirealm::{application::PollStrategy, AttrValue, Attribute, Update};

fn main() {
    // Setup DAPI client
    let mut address_list = AddressList::new();
    address_list.add_uri(rs_dapi_client::Uri::from_static(
        "https://44.239.39.153:1443",
    ));
    address_list.add_uri(rs_dapi_client::Uri::from_static(
        "https://54.149.33.167:1443",
    ));
    address_list.add_uri(rs_dapi_client::Uri::from_static(
        "https://35.164.23.245:1443",
    ));
    address_list.add_uri(rs_dapi_client::Uri::from_static(
        "https://52.33.28.47:1443",
    ));
    let mut sdk = SdkBuilder::new(address_list)
        .with_version(PlatformVersion::get(1).unwrap())
        .with_core("127.0.0.1", 19998, "dashrpc", "password")
        .build()
        .expect("expected to build sdk");

    // Setup model
    let mut model = Model::new(&mut sdk);

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
