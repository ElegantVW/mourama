use std::path::PathBuf;

pub fn data_dir() -> PathBuf {
    if let Ok(p) = std::env::var("MOURAMA_DATA") {
        return PathBuf::from(p);
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".local/share/mourama")
}

pub fn world_db() -> PathBuf {
    data_dir().join("world.sqlite")
}

pub fn web_dir() -> PathBuf {
    if let Ok(p) = std::env::var("MOURAMA_WEB") {
        return PathBuf::from(p);
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let candidates = [
        PathBuf::from(&home).join(".local/lib/faeos/mourama-web"),
        PathBuf::from(&home).join("mourama/web"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("web"),
    ];
    for c in candidates {
        if c.join("index.html").is_file() {
            return c;
        }
    }
    PathBuf::from("web")
}

pub fn ensure_data_dir() -> anyhow::Result<PathBuf> {
    let d = data_dir();
    std::fs::create_dir_all(&d)?;
    Ok(d)
}
