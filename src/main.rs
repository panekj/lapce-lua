use std::{
    fs::{self, File, create_dir_all, remove_file},
    io,
    path::PathBuf, env::set_current_dir,
};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use lapce_plugin::{register_plugin, send_notification, start_lsp, LapcePlugin};

#[derive(Default)]
struct State {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    arch: String,
    os: String,
    configuration: Configuration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Configuration {
    language_id: String,
    options: Option<Value>,
}

register_plugin!(State);

const LSP_NAME: &str = "lua-language-server";
const LSP_VER: &str = "3.3.1";

impl LapcePlugin for State {
    fn initialize(&mut self, info: serde_json::Value) {
        eprintln!("starting lapce-lua");
        let info = serde_json::from_value::<PluginInfo>(info).unwrap();

        // ! We need permission system so we can do stuff like HTTP requests to grab info about
        // ! latest releases, and download them or notify user about updates

        // let response =
        //     futures::executor::block_on(reqwest::get("https://api.github.com/repos/clangd/clangd/releases/latest")).expect("request failed");

        // let api_resp = futures::executor::block_on(response
        //     .json::<GHAPIResponse>()).expect("failed to deserialise api response");

        // let mut download_asset = Asset {
        //     ..Default::default()
        // };
        // for asset in api_resp.assets {
        //     match asset.name.strip_prefix("clangd-") {
        //         Some(asset_name) => match asset_name.starts_with(info.os.as_str()) {
        //             true => download_asset = asset,
        //             false => continue,
        //         },
        //         None => continue,
        //     }
        // }

        // if download_asset.browser_download_url.is_empty() || download_asset.name.is_empty() {
        //     panic!("failed to find clangd in release")
        // }

        // let zip_file = PathBuf::from(download_asset.name);

        let mut zip_file = format!("{LSP_NAME}-{LSP_VER}");

        zip_file = match info.arch.as_str() {
            "x86_64" => match info.os.as_str() {
                "macos" => format!("{zip_file}-darwin-x64.tar.gz"),
                "linux" => format!("{zip_file}-linux-x64.tar.gz"),
                "windows" => format!("{zip_file}-win32-x64.zip"),
                _ => return,
            },
            "aarch64" => match info.os.as_str() {
                "macos" => format!("{zip_file}-darwin-arm64.tar.gz"),
                "linux" => format!("{zip_file}-linux-arm64.tar.gz"),
                _ => return,
            },
            "x86" | "i386" => match info.os.as_str() {
                "windows" => format!("{zip_file}-win32-ia32.zip"),
                _ => return,
            }
            _ => return,
        };

        let zip_file = PathBuf::from(zip_file);

        let download_url = format!(
            "https://github.com/sumneko/lua-language-server/releases/download/{LSP_VER}/{}",
            zip_file.display()
        );

        let mut server_path = PathBuf::from(format!("{LSP_NAME}-{LSP_VER}"));
        
        create_dir_all(&server_path).expect("failed to create lsp dir");

        server_path = server_path.join("bin");

        match info.os.as_str() {
            "windows" => {
                server_path = server_path.join(format!("{LSP_NAME}.exe"));
            }
            _ => {
                server_path = server_path.join(format!("{LSP_NAME}"));
            }
        }

        let exec_path = format!("{}", server_path.display());

        eprintln!("LSP server path: {}", exec_path);

        let lock_file = PathBuf::from("download.lock");
        send_notification(
            "lock_file",
            &json!({
                "path": &lock_file,
            }),
        );

        if !server_path.exists() {
            eprintln!("downloading '{download_url}' to '{}'", zip_file.display());

            if zip_file.exists() {
                match remove_file(&zip_file) {
                    Ok(_) => {},
                    Err(e) => {
                        eprintln!("failed to remove file: {}", e)
                    }
                };
            }

            send_notification(
                "download_file",
                &json!({
                    // "url": download_asset.browser_download_url,
                    "url": download_url,
                    "path": zip_file,
                }),
            );

            assert!(zip_file.exists());

            let mut zip =
                zip::ZipArchive::new(File::open(&zip_file).unwrap()).expect("failed to open zip");

            set_current_dir(format!("{LSP_NAME}-{LSP_VER}")).expect("failed to change dir");

            for i in 0..zip.len() {
                let mut file = zip.by_index(i).unwrap();
                let outpath = match file.enclosed_name() {
                    Some(path) => path.to_owned(),
                    None => continue,
                };

                if (*file.name()).ends_with('/') {
                    create_dir_all(&outpath).unwrap();
                } else {
                    if let Some(p) = outpath.parent() {
                        if !p.exists() {
                            create_dir_all(&p).unwrap();
                        }
                    }
                    let mut outfile = fs::File::create(&outpath).unwrap();
                    io::copy(&mut file, &mut outfile).unwrap();
                }
                eprintln!("created path: {}", outpath.display());
            }

            send_notification(
                "make_file_executable",
                &json!({
                    "path": exec_path,
                }),
            );

            match remove_file(&zip_file) {
                Ok(_) => {},
                Err(e) => {
                    eprintln!("failed to remove file: {}", e)
                }
            };
        }
        _ = remove_file(&lock_file);

        // ! Need to figure out how the sandbox works to use clangd
        // ! provided by system (package managers, etc.)

        // match env::var_os("PATH") {
        //     Some(paths) => {
        //         for path in env::split_paths(&paths) {
        //             if let Ok(dir) = std::path::Path::new(path.as_path()).read_dir() {
        //                 for file in dir.flatten() {
        //                     if let Ok(server) = file.file_name().into_string() {
        //                         if server == server_path {
        //                             server_path = format!("{}", file.path().display())
        //                         }
        //                     }
        //                 }
        //             }
        //         }
        //     }
        //     None => {}
        // }

        start_lsp(&exec_path, "lua", info.configuration.options);
    }
}

// // GitHub API response
// #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
// pub struct GHAPIResponse {
//     pub url: String,
//     pub assets_url: String,
//     pub upload_url: String,
//     pub html_url: String,
//     pub id: i64,
//     pub author: Option<Value>,
//     pub node_id: String,
//     pub tag_name: String,
//     pub target_commitish: String,
//     pub name: String,
//     pub draft: bool,
//     pub prerelease: bool,
//     pub created_at: Option<Value>,
//     pub published_at: Option<Value>,
//     pub assets: Vec<Asset>,
//     pub tarball_url: String,
//     pub zipball_url: String,
//     pub body: String,
// }

// #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
// pub struct Asset {
//     pub url: String,
//     pub id: i64,
//     pub node_id: String,
//     pub name: String,
//     pub label: String,
//     pub uploader: Option<Value>,
//     pub content_type: String,
//     pub state: String,
//     pub size: i64,
//     pub download_count: i64,
//     pub created_at: String,
//     pub updated_at: String,
//     pub browser_download_url: String,
// }
