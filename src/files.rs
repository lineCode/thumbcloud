use pretty_bytes::converter::convert;
use serde_json;
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
struct FileItem {
    name: String,
    size: String,
}

impl FileItem {
    fn from(file_name: String, bytes: u64) -> FileItem {
        FileItem {
            name: file_name,
            size: convert(bytes as f64).replace(" B", " bytes"),
        }
    }

    fn from_name(file_name: String) -> FileItem {
        FileItem {
            name: file_name,
            size: String::new(),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct FileRespond {
    action: String,
    path: String,
    folders: Vec<FileItem>,
    files: Vec<FileItem>,
}

impl FileRespond {
    fn new() -> FileRespond {
        FileRespond {
            action: "sendFilelist".to_string(),
            path: String::new(),
            folders: Vec::new(),
            files: Vec::new(),
        }
    }
}

pub fn get_file_respond(path: PathBuf) -> String {
    let entries = match fs::read_dir(&path) {
        Ok(e) => e,
        Err(_) => {
            return json!({
                "action": "sendError",
                "message": format!("Cannot read the given path: {:?}", path)
            }).to_string();
        }
    };

    let mut respond = FileRespond::new();
    respond.path = path.to_str().unwrap_or("").to_string();

    for entry in entries {
        if let Ok(entry) = entry {
            if let Ok(file_type) = entry.file_type() {
                if let Ok(file_name) = entry.file_name().into_string() {
                    if file_type.is_dir() {
                        respond.folders.push(FileItem::from_name(file_name));
                    } else {
                        let item: FileItem;

                        if let Ok(meta) = entry.metadata() {
                            item = FileItem::from(file_name, meta.len())
                        } else {
                            item = FileItem::from_name(file_name);
                        }

                        respond.files.push(item);
                    }
                }
            }
        }
    }

    serde_json::to_string(&respond).unwrap_or(
        json!({
                "action": "sendError",
                "message": "Cannot parse content"
            }).to_string(),
    )
}
