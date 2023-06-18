use std::{net::{TcpListener, TcpStream, SocketAddr, Ipv4Addr, IpAddr}, io::{Write, Read, ErrorKind}, sync::{atomic::AtomicBool, Arc}, thread, time::Duration};

use tauri::{Window, api::dialog};

use crate::CONNECTED;


const LOCALHOST: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
const ADDRESS_SET: [SocketAddr; 3] = [
    SocketAddr::new(LOCALHOST, 8523),
    SocketAddr::new(LOCALHOST, 7522),
    SocketAddr::new(LOCALHOST, 9233)
];

#[derive(Debug, PartialEq)]
pub enum WindowType {
    MAINWINDOW,
    POPUP
}

#[derive(Debug, PartialEq)]
pub enum CommandSet {
    ISALIVE,
    SUCCESS,
    OPEN(WindowType),
}

impl CommandSet {
    fn from_bytes(buffer: &[u8]) -> Option<Self> {
        match buffer {
            &[0, 0] => {
                Some(CommandSet::SUCCESS)
            }
            &[1, 1] => {
                Some(CommandSet::OPEN(WindowType::MAINWINDOW))
            }
            &[1, 2] => {
                Some(CommandSet::OPEN(WindowType::POPUP))
            }
            &[3, 1] => {
                Some(CommandSet::ISALIVE)
            }
            _ => {
                None
            }
        }
    }

    fn to_bytes<'a>(&self) -> &'a [u8] {
        match &self {
            CommandSet::SUCCESS => {
                &[0, 0]
            },
            CommandSet::OPEN(window_type) => {
                match window_type {
                    WindowType::MAINWINDOW => {
                        &[1, 1]
                    },
                    WindowType::POPUP => {
                        &[1, 2]
                    },
                }
            },
            CommandSet::ISALIVE => {
                &[3, 1]
            }
        }
    }
}

pub fn send(command: CommandSet) -> Option<CommandSet> {
    let mut selected_address = 0;
    loop {
        if let Ok(mut stream) = TcpStream::connect_timeout(&ADDRESS_SET[selected_address], Duration::from_millis(200)) {
            stream.write(command.to_bytes()).expect("Failed sending command");
    
            let mut buffer = [0u8; 2];
            stream.read(&mut buffer).expect("Failed reading buffer");
            
            if let Some(command) = CommandSet::from_bytes(&buffer) {
                return Some(command);
            }
        } else {
            selected_address += 1;
            if selected_address >= ADDRESS_SET.len() {
                break;
            }
        }
    }
    None
}

pub fn listen(main_window: Window, popup_window: Window, splash_window: Window) {
    let listener = TcpListener::bind(&ADDRESS_SET[..]).expect("Failed binding listener");

    for stream in listener.incoming() {
        if let Ok(mut stream) = stream {
            println!("Received a message");
            let mut buffer = [0u8; 2];
            stream.read(&mut buffer).expect("Failed reading buffer");
            
            if let Some(command) = CommandSet::from_bytes(&buffer) {
                match command {
                    CommandSet::OPEN(window_type) => {
                        match window_type {
                            WindowType::MAINWINDOW => {
                                if !main_window.is_visible().unwrap() {
                                    let is_connected = unsafe { CONNECTED };

                                    if !is_connected {
                                        dialog::message(Some(&splash_window), "Not loaded", "Application have not been loaded yet");
                                        return;
                                    }
                                    
                                    main_window.show().unwrap();
                                }
                            },
                            WindowType::POPUP => {
                                let is_connected = unsafe { CONNECTED };

                                if !is_connected {
                                    dialog::message(Some(&splash_window), "Not loaded", "Application have not been loaded yet");
                                    return;
                                }

                                if !popup_window.is_visible().unwrap() {
                                    popup_window.show().unwrap();
                                }
                            },
                        }
                    },
                    _ => {}
                }
            }

            stream.write(CommandSet::SUCCESS.to_bytes()).expect("Failed sending response");
        }
    }
}

#[derive(Debug)]
pub enum ListenerType {
    SIGNIN,
    PAYMENT
}

pub fn new_http_server(listen_type: ListenerType, main_window: Window, popup_window: Window) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed starting the http server");
    dbg!(&listen_type);
    dbg!(listener.local_addr().unwrap().to_string());
    main_window.eval(format!(r#"window.dispatchEvent(new CustomEvent("updateCallbackAddress", {{ bubbles: true, detail: {{ url: "{}" }} }} ))"#, listener.local_addr().unwrap().to_string()).as_str()).unwrap();

    listener.set_nonblocking(true).unwrap();

    let should_stop = Arc::new(AtomicBool::new(false));

    let ss = should_stop.clone();
    let mw = main_window.clone();

    match listen_type {
        ListenerType::PAYMENT => { },
        _ => {
            thread::spawn(move || {
                thread::sleep(Duration::from_secs(30));
                println!("Timeout");
                ss.store(true, std::sync::atomic::Ordering::Relaxed);
                mw.eval(r#"window.dispatchEvent(new CustomEvent("listenerTimeout"))"#).unwrap();
            });
        }
    }

    for stream in listener.incoming() {
        if should_stop.load(std::sync::atomic::Ordering::Relaxed) {
            break;
        }

        match stream {
            Ok(mut stream) => {
                let mut head = String::new();
                println!("Received a message");
    
                loop {
                    let mut buffer = [0u8; 1024];
                    let len = stream.read(&mut buffer).unwrap();
    
                    head.push_str(String::from_utf8_lossy(&buffer[..len]).to_string().as_str());
    
                    if len < buffer.len() {
                        break;
                    }
                }

                let body;

                match listen_type {
                    ListenerType::SIGNIN => {
                        let access_token = head.split_once("access_token=").unwrap().1.split_once("&").unwrap().0;
                        let refresh_token = head.split_once("refresh_token=").unwrap().1.split_once("&").unwrap().0;
            
                        main_window.eval(format!(r#"window.dispatchEvent(new CustomEvent("doneSignIn", {{ bubbles: true, detail: {{ access_token: "{access_token}", refresh_token: "{refresh_token}" }} }} ))"#).as_str()).unwrap();
                        popup_window.eval("location.reload()").unwrap();

                        body = "You have successfully logged-in";
                    },
                    ListenerType::PAYMENT => {
                        main_window.eval("location.reload()").unwrap();
                        popup_window.eval("location.reload()").unwrap();

                        body = "The payment was successfully made"
                    },
                }

                let body = r#"<!DOCTYPE html><html lang="en"> <head> <meta charset="UTF-8"/> <meta http-equiv="X-UA-Compatible" content="IE=edge"/> <meta name="viewport" content="width=device-width, initial-scale=1.0"/> <title>QuickGPT | Success</title> <link rel="preconnect" href="https://fonts.googleapis.com"/> <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin/> <link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;700;900&display=swap" rel="stylesheet"/> <style>body{margin: 0; background-color: #343541; color: #fff; overflow: hidden; font-family: "Inter", sans-serif;}.wrapper{width: 100vw; height: 100vh; display: flex; justify-content: center; align-items: center; flex-direction: column; font-size: large;}.wrapper > h1{font-size: 3rem; line-height: 1; font-weight: 900;}p{margin: 0;}</style> </head> <body> <div class="wrapper"> <h1>QuickGPT</h1> <p>{message}</p><p> Please open <strong>QuickGPT</strong> window to continue </p></div></body></html>"#
                    .replace("{message}", body);

                let content_len = format!("Content-Length: {}", body.len());
                
                let mut response = Vec::new();
                response.push("HTTP/1.1 200 OK");
                response.push(&content_len);
                response.push("Content-Type: text/html");
                response.push("");
                response.push(body.as_str());
    
                stream.write(response.join("\n").as_bytes()).unwrap();
                break;
            }
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(500));
                continue;
            }
            Err(e) => {
                panic!("error: {}", e);
            }
        }
    }

}
