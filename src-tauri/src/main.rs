// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod com;
mod configuration;

use std::{thread, time::Duration};

use com::new_http_server;
use enigo::{Enigo, MouseControllable};
use tauri::{
    api::{dialog, shell},
    App, AppHandle, GlobalShortcutManager, Manager, Position, SystemTray, SystemTrayEvent,
    SystemTrayMenu, Window, WindowEvent,
};

use crate::configuration::{BANK_URL, KEYBIND};

#[cfg(dev)]
const URL: &str = "http://localhost:3000";

#[cfg(not(dev))]
const URL: &str = "https://quickgpt-ui.vercel.app";

static mut CONNECTED: bool = false;
static mut SIGNED_IN: bool = false;

macro_rules! change_location_eval {
    ($uri:expr,$w:expr) => {
        $w.eval(&format!("window.location.replace('{}')", $uri))
            .expect("Failed changing location")
    };
}

macro_rules! make_menu_item {
    ($id:expr,$title:expr) => {
        tauri::CustomMenuItem::new($id.to_string(), $title)
    };
}

fn get_centered_mouse_pos(width: f64, height: f64) -> (f64, f64) {
    let enigo = Enigo::new();
    let (mouse_x, mouse_y) = enigo.mouse_location();

    let mouse_x = mouse_x as f64;
    let mouse_y = mouse_y as f64;

    let window_position_x = mouse_x - (width / 2.0);
    let window_position_y = mouse_y - (height / 2.0);

    (window_position_x, window_position_y)
}

fn popup(handle: AppHandle) -> Window {
    let window_width = 400.0;
    let window_height = 600.0;

    let (window_position_x, window_position_y) =
        get_centered_mouse_pos(window_width, window_height);

    let popup_window = tauri::WindowBuilder::new(
        &handle,
        "popup",
        tauri::WindowUrl::External(URL.parse().unwrap()),
    )
    .title("QuickGPT Mini")
    .resizable(false)
    .always_on_top(true)
    .decorations(true)
    .user_agent("POPUP")
    .transparent(true)
    .visible(false)
    .inner_size(window_width, window_height)
    .position(window_position_x, window_position_y)
    .build()
    .unwrap();

    popup_window
}

fn app(app: &mut App) -> Result<(), Box<(dyn std::error::Error + 'static)>> {
    let main_window = app
        .get_window("main")
        .expect("Failed fetching the main window");
    let splash_window = app
        .get_window("splash")
        .expect("Failed fetching the splash window");

    let popup_window = popup(app.handle());
    popup_window.eval("location.reload()").unwrap();

    let popup = popup_window.clone();
    app.global_shortcut_manager()
        .register(KEYBIND, move || {
            if popup.is_visible().unwrap() {
                popup.hide().unwrap();
            } else {
                let is_connected = unsafe { CONNECTED };
                let is_signedin = unsafe { SIGNED_IN };

                if !is_signedin {
                    dialog::message(Some(&popup), "Not signed in", "You have not signed in yet");
                    return;
                }

                if !is_connected {
                    dialog::message(
                        Some(&popup),
                        "Not loaded",
                        "Application have not been loaded yet",
                    );
                    return;
                }

                let window_size = popup.inner_size().unwrap();
                let window_position =
                    get_centered_mouse_pos(window_size.width as f64, window_size.height as f64);

                popup
                    .set_position(Position::Physical(tauri::PhysicalPosition {
                        x: window_position.0 as i32,
                        y: window_position.1 as i32,
                    }))
                    .unwrap();
                popup.show().unwrap();
            }
        })
        .unwrap_or_default();

    if let Some(command) = com::send(com::CommandSet::ISALIVE) {
        if command == com::CommandSet::SUCCESS {
            com::send(com::CommandSet::OPEN(com::WindowType::MAINWINDOW));
            std::process::exit(0);
        }
    }

    let tray_menu = SystemTrayMenu::new()
        .add_item(make_menu_item!("open", "Open"))
        .add_item(make_menu_item!("quit", "Quit"));
    let tray = SystemTray::new().with_menu(tray_menu);

    tray.build(app).expect("Failed building the system tray");

    splash_window.show().unwrap();

    let mw = main_window.clone();
    let pw = popup_window.clone();
    let sw = splash_window.clone();
    thread::spawn(move || {
        com::listen(mw, pw, sw);
    });

    let popup = popup_window.clone();
    let mw = main_window.clone();
    popup.listen("conversationSend", move |_| {
        mw.eval("location.reload()").unwrap();
    });

    let popup = popup_window.clone();
    main_window.listen("conversationSend", move |_| {
        popup.eval("location.reload()").unwrap();
    });

    let mw = main_window.clone();
    let splash = splash_window.clone();
    main_window.listen("DOMContentLoaded", move |e| {
        let page = e.payload().unwrap_or("").replace("\"", "");

        match page.as_str() {
            "login" => unsafe {
                SIGNED_IN = false;
            },
            "chats" => unsafe {
                SIGNED_IN = true;
            },
            _ => {}
        }

        if let Ok(visible) = splash.is_visible() {
            if visible {
                splash.close().expect("Failed closing the splash window");
                mw.show().expect("Failed showing the main window");
            }
        }

        unsafe {
            CONNECTED = true;
        }
    });

    let mw = main_window.clone();
    let popup = popup_window.clone();
    main_window.listen("requestSignIn", move |_| {
        let mw = mw.clone();
        let popup = popup.clone();
        thread::spawn(move || {
            new_http_server(com::ListenerType::SIGNIN, mw.clone(), popup.clone())
        });
    });

    let mw = main_window.clone();
    let popup = popup_window.clone();
    main_window.listen("requestPayment", move |_| {
        let mw = mw.clone();
        let popup = popup.clone();
        thread::spawn(move || {
            new_http_server(com::ListenerType::PAYMENT, mw.clone(), popup.clone())
        });
    });

    let mw = main_window.clone();
    main_window.listen("signIn", move |e| {
        let login_url = e.payload().expect("Invalid URL").replace("\"", "");
        shell::open(&mw.shell_scope(), login_url, None).expect("Failed opening login url");
    });

    let popup = popup_window.clone();
    main_window.listen("signOut", move |_| {
        popup.hide().unwrap();
        popup.eval("location.reload()").unwrap();
    });

    let mw = main_window.clone();
    main_window.listen("openBank", move |_| {
        shell::open(&mw.shell_scope(), BANK_URL, None).expect("Failed opening bank url");
    });

    let mw = main_window.clone();
    main_window.listen("openStripe", move |e| {
        let stripe_link = e.payload().expect("Invalid URL").replace("\"", "");
        shell::open(
            &mw.shell_scope(),
            format!("https://buy.stripe.com/{stripe_link}"),
            None,
        )
        .expect("Failed opening login url");
    });

    let mw = main_window.clone();
    main_window.listen("openLink", move |e| {
        let link = e.payload().expect("Invalid URL").replace("\"", "");
        shell::open(&mw.shell_scope(), link, None).expect("Failed opening login url");
    });

    let popup = popup_window.clone();
    main_window.listen("refreshPopup", move |_| {
        popup
            .eval("location.reload()")
            .expect("Failed refreshing the popup URL");
    });

    change_location_eval!(URL, main_window);

    thread::spawn(move || {
        thread::sleep(Duration::from_secs(20));
        let is_connected = unsafe { CONNECTED };

        if !is_connected {
            splash_window.emit("failed", "").unwrap();
            thread::sleep(Duration::from_secs(5));
            std::process::exit(0);
        }
    });

    Ok(())
}

fn system_tray_event(app: &AppHandle, event: SystemTrayEvent) {
    let main_window = app
        .get_window("main")
        .expect("Failed fetching the main window");
    let splash_window = app.get_window("splash");

    match event {
        SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
            "open" => {
                let is_connected = unsafe { CONNECTED };

                if !is_connected {
                    if let Some(window) = splash_window {
                        dialog::message(
                            Some(&window),
                            "Not loaded",
                            "Application have not been loaded yet",
                        );
                    }
                    return;
                }

                if !main_window.is_visible().unwrap() {
                    main_window.show().unwrap();
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        },
        _ => {}
    }
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_positioner::init())
        .invoke_handler(tauri::generate_handler![])
        .on_system_tray_event(system_tray_event)
        .on_window_event(|event| match event.event() {
            WindowEvent::CloseRequested { api, .. } => {
                event.window().hide().unwrap();
                api.prevent_close();
            }
            _ => {}
        })
        .setup(app)
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
