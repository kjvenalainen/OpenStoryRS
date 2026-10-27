fn debug_ui_stage(stage: &str) {
    if std::env::var("OPENSTORY_DEBUG_NETWORK").as_deref() == Ok("1") {
        let message = format!("OpenStory UI: {stage}");
        eprintln!("{message}");
        if let Some(path) = std::env::var_os("OPENSTORY_DEBUG_LOG") {
            if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
                let _ = writeln!(file, "{message}");
            }
        }
    }
}
