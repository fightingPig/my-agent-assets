use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AppInfo {
    name: &'static str,
    version: &'static str,
    platform: &'static str,
    arch: &'static str,
    backend_ready: bool,
}

#[tauri::command]
fn app_info() -> AppInfo {
    AppInfo {
        name: "My Agent Assets",
        version: env!("CARGO_PKG_VERSION"),
        platform: std::env::consts::OS,
        arch: std::env::consts::ARCH,
        backend_ready: true,
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![app_info])
        .run(tauri::generate_context!())
        .expect("error while running My Agent Assets");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_info_is_read_only_platform_metadata() {
        let info = app_info();
        assert_eq!(info.name, "My Agent Assets");
        assert!(info.backend_ready);
    }
}
