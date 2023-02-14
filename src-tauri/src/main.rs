#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use serialport::SerialPortInfo;
use tauri::{
    CustomMenuItem, Manager, SystemTray, SystemTrayEvent, SystemTrayMenu, SystemTrayMenuItem,
};



// TODO: add a port selector
// #[derive(serde::Serialize)]
// struct PortInfo {
//     name: String,
//     description: String,
//     hardware_id: String,
//     port_type: String,
// }

// #[tauri::command]
fn get_ports() -> Vec<SerialPortInfo> {
    return serialport::available_ports().expect("No ports found!");
}

#[tauri::command]
fn get_s8_value(state: State<Port>) -> u16 {
    // get the port from the state
    let port_string = state.0.lock().unwrap().clone();
    // println!("Port: {}", port_string);



    let mut port = serialport::new(port_string.as_str(), 9600)
        .timeout(std::time::Duration::from_millis(10))
        .open()
        .expect("Failed to open port");

    // flush the port and send "\xFE\x44\x00\x08\x02\x9F\x25"
    port.flush().expect("Failed to flush port");
    // read the response 7 byte and return the value
    port.write(&[0xFE, 0x44, 0x00, 0x08, 0x02, 0x9F, 0x25])
        .expect("Failed to write to port");
    // sleep for 100ms
    std::thread::sleep(std::time::Duration::from_millis(100));
    // read the response
    let mut buf = [0; 7];
    port.read(&mut buf).expect("Failed to read from port");
    // response is 7 bytes, high byte is index 3, low byte is index 4
    let high_byte = buf[3];
    let low_byte = buf[4];
    // calculate the value
    let value = (high_byte as u16) * 256 + (low_byte as u16);
    // return the value
    value
}

fn print_ports(ports: &Vec<SerialPortInfo>) {
    println!("found ports:\n{}", 
    ports
        .iter()
        .map(|p| p.port_name.clone())
        .collect::<Vec<String>>().join("\n"));
}

use tauri::State;
use std::sync::Mutex;
use std::process::exit;

struct Port(Mutex<String>);

fn main() {
    let quit = CustomMenuItem::new("quit".to_string(), "Quit");
    let hide = CustomMenuItem::new("hide".to_string(), "Hide");
    let tray_menu = SystemTrayMenu::new()
        .add_item(quit)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(hide);
    // let tray = SystemTray::new().with_menu(tray_menu);

    tauri::Builder::default()
        .system_tray(SystemTray::new().with_menu(tray_menu))
        .on_system_tray_event(|app, event| match event {
            SystemTrayEvent::MenuItemClick { id, .. } => {
                let item_handle = app.tray_handle().get_item(&id);
                match id.as_str() {
                    "hide" => {
                        let window = app.get_window("main").unwrap();
                        if window.is_visible().unwrap() {
                            window.hide().unwrap();
                            // you can also `set_selected`, `set_enabled` and `set_native_image` (macOS only).
                            item_handle.set_title("Show").unwrap();
                        } else {
                            window.show().unwrap();
                            item_handle.set_title("Hide").unwrap();
                        }
                    },
                    "quit" => {
                        app.exit(0)
                    },
                    _ => {}
                }
            }
            _ => {}
        })
        .setup(|app| {
            match app.get_cli_matches() {
                Ok(matches) => {
                    matches.args.get("port").map(|port| {
                        println!("Port: {}", &port.value);

                        let mut port_str = port.value.to_string();

                        // TODO: windows pars is wrapped in quotes, need to test on linux and mac
                        port_str = port_str.replace("\"", "");

                        let port = Port(Mutex::new(port_str));
                        app.manage(port);
                    });
                }
                Err(e) => {
                    println!("Error: {}", e);

                    println!("No port specified, please specify a port with --port <port>");

                    print_ports(&get_ports());

                    exit(1);
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_s8_value])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
