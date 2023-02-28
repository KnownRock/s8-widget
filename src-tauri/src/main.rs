#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use reqwest::header::{HeaderMap, AUTHORIZATION};
use serialport::SerialPortInfo;
use tauri::{
    CustomMenuItem, Manager, SystemTray, SystemTrayEvent, SystemTrayMenu, SystemTrayMenuItem,
};

use std::os::windows::process::CommandExt;
use std::process::Command;



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


fn exec_hooks(s8_value: u16) {
    println!("Executing hooks...");
    println!("S8 value: {}", s8_value);
    println!("Current directory: {}", std::env::current_dir().unwrap().display());

    // traverse hooks folder and execute all files
    let hooks_path = std::path::Path::new("hooks");
    if hooks_path.exists() {
        for entry in std::fs::read_dir(hooks_path).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_file() {
                println!("Executing hook: {}", path.display());
                let output = std::process::Command::new(path)
                    .arg(s8_value.to_string())
                    // TODO: make it cross-platform
                    .creation_flags(0x08000000)
                    .output()
                    .expect("Failed to execute hook");
                println!("Hook output: {}", String::from_utf8_lossy(&output.stdout));
            }
        }
    }


}


#[tauri::command]
fn get_s8_value(state: State<'_, Config>) -> u16 {
    let config = state.0.lock().unwrap();

    let get_value_type = config.get("type").unwrap();

    dbg!(get_value_type);

    

    if get_value_type == "serial" {
            
        // get the port from the state
        // let port_string = state.0.lock().unwrap().clone();
        // println!("Port: {}", port_string);
        let port_string = config.get("port").unwrap().as_str().unwrap();



        let mut port = serialport::new(port_string, 9600)
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
        // call the hooks
        exec_hooks(value);
        // return the value
        value

    } else if get_value_type == "http-get" {

        // let url = state.0.lock().unwrap().get("url").unwrap();
        // let header = state.0.lock().unwrap().get("header").unwrap();
        let url = config.get("url").unwrap();
        let header = config.get("header").unwrap();

        let mut hash_map_headers = HeaderMap::new();
        
        // for (key, value) in header.as_object().unwrap() {
        //     hash_map_headers.insert(key, value.as_str().unwrap().parse().unwrap());
        // }

        hash_map_headers.insert(AUTHORIZATION, header["Authorization"].as_str().unwrap().parse().unwrap());
        

        // dbg!(header);
        let client = reqwest::blocking::Client::new();
        let res = client.get(url.as_str().unwrap())
            .headers(hash_map_headers)
            .send()
            .expect("Failed to send request");
        let key = config.get("key").unwrap().as_str().unwrap();
        let res_text = res.text().unwrap();
        let res_json: serde_json::Value = serde_json::from_str(&res_text).unwrap();
        // TODO: add error handling and get value with key chain
        let value_text = res_json[key].as_str().unwrap();
        let value = value_text.parse::<u16>().unwrap();
        // call the hooks
        exec_hooks(value);
        value

    } else {
        0
    }

}

fn print_ports(ports: &Vec<SerialPortInfo>) {
    println!("found ports:\n{}", 
    ports
        .iter()
        .map(|p| p.port_name.clone())
        .collect::<Vec<String>>().join("\n"));
}

use tauri::State;
use std::collections::HashMap;
use std::sync::Mutex;
use std::process::exit;

struct Port(Mutex<String>);
struct Config(Mutex<HashMap<String, serde_json::Value>>);

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
                    let mut have_port_flag = false;
                    matches.args.get("port").map(|port| {
                        println!("Port: {}", &port.value);
                        if &port.value != false {
                            let mut port_str = port.value.to_string();

                            // TODO: windows pars is wrapped in quotes, need to test on linux and mac
                            port_str = port_str.replace("\"", "");
    
                            // let port = Port(Mutex::new(port_str));
                            // app.manage(port);
    
                            let config = Config(Mutex::new(HashMap::new()));
                            config.0.lock().unwrap().insert("port".to_string(), serde_json::Value::String(port_str));
                            config.0.lock().unwrap().insert("type".to_string(), serde_json::Value::String("serial".to_string()));
    
                            println!("State managed");
                            app.manage(config);
                        }

                        have_port_flag = true;
                    });

                    if !have_port_flag {
                        matches.args.get("config").map(|config| {
                            println!("Config: {}", &config.value);

                            let config_file_path_raw = config.value.to_string();

                            // dbg!(config_file_path.get(config_file_path.len() - 1..));
                            let config_file_path = config_file_path_raw.replace("\"", "");
                            
                            println!("Config file path: {}", &config_file_path);

                            let config_state = Config(Mutex::new(HashMap::new()));

                            // read file
                            let file = std::fs::read_to_string(config_file_path).expect("Unable to read file");

                            // parse file
                            let config_map: HashMap<String, serde_json::Value > = serde_json::from_str(&file).expect("Unable to parse file");

                            // add config to state
                            config_state.0.lock().unwrap().extend(config_map);

                            println!("State managed");
                            app.manage(config_state);
                        });
                    }

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
