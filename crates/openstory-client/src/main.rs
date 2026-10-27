#![forbid(unsafe_code)]

#[cfg(not(target_arch = "wasm32"))]
mod session;

#[cfg(not(target_arch = "wasm32"))]
mod native;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> std::process::ExitCode {
    match native::run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("OpenStory: {message}");
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn main() {
    eprintln!("The browser entry point and transport adapter are not implemented yet.");
}
