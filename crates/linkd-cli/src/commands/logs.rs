use linkd_core::log_path;

pub async fn run(follow: bool) -> anyhow::Result<()> {
    let path = log_path();
    if !path.exists() {
        println!("No log file yet at {}", path.display());
        return Ok(());
    }

    if follow {
        println!("Following {} (Ctrl+C to stop)", path.display());
        let mut last_len = 0u64;
        loop {
            if let Ok(meta) = std::fs::metadata(&path) {
                let len = meta.len();
                if len > last_len {
                    let data = std::fs::read(&path)?;
                    let slice = &data[last_len as usize..];
                    print!("{}", String::from_utf8_lossy(slice));
                    last_len = len;
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    } else {
        let data = std::fs::read_to_string(&path)?;
        print!("{data}");
        Ok(())
    }
}
