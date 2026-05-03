mod colors;
mod config;

use crate::config::VaporConfig;
use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, glib, gio};
use gtk4_layer_shell::{Layer, Edge, LayerShell};
use webkit6::prelude::*;
use webkit6::WebView;
use std::rc::Rc;
use std::cell::RefCell;
use crate::colors::PywalColors;
use notify::{Watcher, RecursiveMode, event};
use serde::Serialize;

use std::process::{Command, id};
use std::env;

#[derive(Serialize, Clone)]
pub struct MonitorPayload {
    pub mode: String,
    pub total_width: i32,
    pub total_height: i32,
    pub view_x: i32,
    pub view_y: i32,
    pub view_width: i32,
    pub view_height: i32,
}

fn handle_processes(is_reload: bool) {
    let current_pid = id();
    
    let output = Command::new("pgrep")
        .arg("-x")
        .arg("vaporwall")
        .output()
        .unwrap_or_else(|_| std::process::Output { 
            status: std::os::unix::process::ExitStatusExt::from_raw(0), 
            stdout: Vec::new(), 
            stderr: Vec::new() 
        });
        
    let output_str = String::from_utf8_lossy(&output.stdout);
    let mut other_pids = Vec::new();
    
    for line in output_str.lines() {
        if let Ok(pid) = line.parse::<u32>() {
            if pid != current_pid {
                other_pids.push(pid);
            }
        }
    }
    
    if is_reload {
        if other_pids.is_empty() {
            println!("No running vaporwall instance found to reload.");
            std::process::exit(1);
        }
        for pid in other_pids {
            let _ = Command::new("kill").arg("-SIGUSR1").arg(pid.to_string()).status();
        }
        println!("Reload signal sent to existing vaporwall processes.");
        std::process::exit(0);
    } else {
        for pid in other_pids {
            let _ = Command::new("kill").arg("-9").arg(pid.to_string()).status();
        }
    }
}

fn main() -> glib::ExitCode {
    let args: Vec<String> = env::args().collect();
    let is_reload = args.iter().any(|arg| arg == "-r");
    handle_processes(is_reload);

    let app = Application::builder()
        .application_id("com.vaporwall.engine")
        .flags(gio::ApplicationFlags::NON_UNIQUE)
        .build();

    app.connect_activate(build_ui);
    app.run_with_args(&["vaporwall"])
}

fn build_ui(app: &Application) {
    let vapor_config = VaporConfig::load_or_create().unwrap_or_default();
    let config_rc = Rc::new(RefCell::new(vapor_config));

    let colors = Rc::new(RefCell::new(PywalColors::load().unwrap_or_else(|_| {
        PywalColors {
            special: crate::colors::SpecialColors {
                background: "#1a1a1a".to_string(),
                foreground: "#ffffff".to_string(),
                cursor: "#ffffff".to_string(),
            },
            colors: std::collections::HashMap::new(),
        }
    })));

    let display = gtk4::gdk::Display::default().expect("Could not get default display");
    let monitors_list = display.monitors();
    let n_monitors = monitors_list.n_items();
    
    let mut selected_monitors = Vec::new();
    let displays_conf = config_rc.borrow().monitor.displays.clone();
    
    for i in 0..n_monitors {
        if let Some(item) = monitors_list.item(i) {
            if let Ok(monitor) = item.downcast::<gtk4::gdk::Monitor>() {
                if let Some(conn) = monitor.connector() {
                    println!("Found monitor: {}", conn);
                } else {
                    println!("Found monitor with NO connector string");
                }
                
                if displays_conf == "all" {
                    selected_monitors.push(monitor);
                } else if let Some(conn) = monitor.connector() {
                    let displays_list: Vec<&str> = displays_conf.split(',').map(|s| s.trim()).collect();
                    if displays_list.contains(&conn.as_str()) {
                        selected_monitors.push(monitor);
                    }
                }
            }
        }
    }
    
    println!("Selected {} monitors.", selected_monitors.len());

    let mut min_x = i32::MAX;
    let mut min_y = i32::MAX;
    let mut max_x = i32::MIN;
    let mut max_y = i32::MIN;
    
    for mon in &selected_monitors {
        let geo = mon.geometry();
        min_x = min_x.min(geo.x());
        min_y = min_y.min(geo.y());
        max_x = max_x.max(geo.x() + geo.width());
        max_y = max_y.max(geo.y() + geo.height());
    }
    
    let total_width = if max_x > min_x { max_x - min_x } else { 0 };
    let total_height = if max_y > min_y { max_y - min_y } else { 0 };
    let mode = config_rc.borrow().monitor.mode.clone();
    
    let mut center_offset_x = 0;
    let center_display = config_rc.borrow().monitor.center_display.clone();
    if mode == "span" && center_display != "auto" {
        for mon in &selected_monitors {
            if let Some(conn) = mon.connector() {
                if conn == center_display {
                    let geo = mon.geometry();
                    let mon_center = geo.x() - min_x + geo.width() / 2;
                    let target_center = total_width / 2;
                    center_offset_x = mon_center - target_center;
                }
            }
        }
    }

    let views_rc = Rc::new(RefCell::new(Vec::new()));

    for monitor in selected_monitors {
        let geo = monitor.geometry();
        
        let payload = if mode == "span" {
            MonitorPayload {
                mode: "span".to_string(),
                total_width,
                total_height,
                view_x: (geo.x() - min_x) - center_offset_x,
                view_y: geo.y() - min_y,
                view_width: geo.width(),
                view_height: geo.height(),
            }
        } else {
            MonitorPayload {
                mode: "duplicate".to_string(),
                total_width: geo.width(),
                total_height: geo.height(),
                view_x: 0,
                view_y: 0,
                view_width: geo.width(),
                view_height: geo.height(),
            }
        };

        let window = ApplicationWindow::builder()
            .application(app)
            .title("Vaporwall")
            .build();

        window.init_layer_shell();
        window.set_layer(Layer::Background);
        let namespace = format!("vaporwall-{}", monitor.connector().unwrap_or_else(|| "unknown".into()));
        window.set_namespace(&namespace);
        window.set_monitor(&monitor);
        
        window.set_anchor(Edge::Top, true);
        window.set_anchor(Edge::Bottom, true);
        window.set_anchor(Edge::Left, true);
        window.set_anchor(Edge::Right, true);
        window.set_exclusive_zone(-1);
        window.set_margin(Edge::Top, -5);
        window.set_margin(Edge::Bottom, -5);
        window.set_margin(Edge::Left, -5);
        window.set_margin(Edge::Right, -5);

        let webview = WebView::new();
        let settings = WebViewExt::settings(&webview).unwrap();
        settings.set_allow_file_access_from_file_urls(true);
        settings.set_allow_universal_access_from_file_urls(true);
        webview.set_background_color(&gtk4::gdk::RGBA::new(0.0, 0.0, 0.0, 0.0));

        window.set_child(Some(&webview));

        let mut path = VaporConfig::walls_dir();
        path.push(&config_rc.borrow().core.current_wall);
        webview.load_uri(&format!("file://{}", path.to_str().unwrap()));

        let colors_clone = colors.clone();
        let config_clone = config_rc.clone();
        let payload_clone = payload.clone();
        webview.connect_load_changed(move |view, event| {
            if event == webkit6::LoadEvent::Finished {
                inject_state(view, &colors_clone.borrow(), &config_clone.borrow(), &payload_clone);
            }
        });

        window.present();
        
        // Prevent windows from being dropped
        views_rc.borrow_mut().push((window, webview, payload));
    }

    let colors_watcher = colors.clone();
    let config_watcher = config_rc.clone();
    let views_watcher = views_rc.clone();
    let (tx, rx) = glib::MainContext::channel(glib::Priority::DEFAULT);
    
    std::thread::spawn(move || {
        let home = std::env::var("HOME").unwrap();
        let path = std::path::PathBuf::from(home).join(".cache/wal");
        let mut watcher = notify::recommended_watcher(move |res: notify::Result<event::Event>| {
            if let Ok(event) = res {
                if event.paths.iter().any(|p| p.ends_with("colors.json")) {
                    let _ = tx.send(());
                }
            }
        }).unwrap();
        watcher.watch(&path, RecursiveMode::NonRecursive).unwrap();
        loop { std::thread::sleep(std::time::Duration::from_secs(1)); }
    });

    rx.attach(None, move |_| {
        if let Ok(new_colors) = PywalColors::load() {
            *colors_watcher.borrow_mut() = new_colors;
            for (_, view, payload) in views_watcher.borrow().iter() {
                inject_state(view, &colors_watcher.borrow(), &config_watcher.borrow(), payload);
            }
        }
        glib::ControlFlow::Continue
    });

    let config_signal = config_rc.clone();
    let colors_signal = colors.clone();
    let views_signal = views_rc.clone();
    
    glib::unix_signal_add_local(libc::SIGUSR1, move || {
        println!("Received SIGUSR1: Reloading config...");
        if let Ok(new_config) = VaporConfig::load_or_create() {
            let old_wall = config_signal.borrow().core.current_wall.clone();
            *config_signal.borrow_mut() = new_config;
            
            for (_, view, payload) in views_signal.borrow().iter() {
                if old_wall != config_signal.borrow().core.current_wall {
                    let mut path = VaporConfig::walls_dir();
                    path.push(&config_signal.borrow().core.current_wall);
                    view.load_uri(&format!("file://{}", path.to_str().unwrap()));
                } else {
                    inject_state(view, &colors_signal.borrow(), &config_signal.borrow(), payload);
                }
            }
        }
        glib::ControlFlow::Continue
    });
}

fn inject_state(webview: &WebView, colors: &PywalColors, config: &VaporConfig, payload: &MonitorPayload) {
    let mut config_json: serde_json::Value = serde_json::to_value(config).unwrap_or(serde_json::json!({}));
    if let serde_json::Value::Object(ref mut map) = config_json {
        let payload_json = serde_json::to_value(payload).unwrap_or(serde_json::json!({}));
        map.insert("monitor_data".to_string(), payload_json);
    }
    let config_json_str = serde_json::to_string(&config_json).unwrap_or_else(|_| "{}".to_string());
    
    let js = format!(
        r#"
        (function() {{
            window.vaporConfig = {};
            window.vaporColors = {{
                background: {},
                color1: {},
                color2: {},
                color3: {},
                color4: {},
                color5: {},
                color6: {},
                color7: {}
            }};

            const event = new CustomEvent('vaporwall:update', {{
                detail: {{
                    config: window.vaporConfig,
                    colors: window.vaporColors
                }}
            }});
            window.dispatchEvent(event);
        }})();
        "#,
        config_json_str,
        hex_to_int(&colors.special.background),
        hex_to_int(colors.colors.get("color1").unwrap_or(&"#ff00ff".to_string())),
        hex_to_int(colors.colors.get("color2").unwrap_or(&"#00ffff".to_string())),
        hex_to_int(colors.colors.get("color3").unwrap_or(&"#00ff00".to_string())),
        hex_to_int(colors.colors.get("color4").unwrap_or(&"#ffffff".to_string())),
        hex_to_int(colors.colors.get("color5").unwrap_or(&"#ff00ff".to_string())),
        hex_to_int(colors.colors.get("color6").unwrap_or(&"#ffff00".to_string())),
        hex_to_int(colors.colors.get("color7").unwrap_or(&"#ff0000".to_string()))
    );
    webview.evaluate_javascript(&js, None, None, None::<&gio::Cancellable>, |_| {});
}

fn hex_to_int(hex: &str) -> u32 {
    let hex = hex.trim_start_matches('#');
    u32::from_str_radix(hex, 16).unwrap_or(0)
}
