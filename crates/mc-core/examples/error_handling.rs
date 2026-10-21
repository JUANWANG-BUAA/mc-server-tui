//! Run with `cargo run -p mc-core --example error_handling`.

use mc_core::{AppError, ErrorCode, ErrorTemplates, Language};

fn main() {
    let error = AppError::new(
        ErrorCode::DiskSpaceLow,
        "Not enough space for a world backup",
    )
    .with_evidence("need 524288000 bytes, have 104857600 bytes")
    .with_action("Free space on the instance volume, then retry the backup.")
    .retryable();
    println!(
        "{}",
        error.to_error_message(Language::ZhCN).diagnostic_report()
    );
    println!(
        "{}",
        ErrorTemplates::port_in_use(25565, Language::En).diagnostic_report()
    );
}
