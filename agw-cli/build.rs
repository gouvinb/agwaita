//! Build script for Agwaita CLI.
//!
//! Computes `AGW_VERSION` from Git metadata at build time and exposes it
//! to the compiler as an environment variable.

#[path = "build/version.rs"]
mod version;

use sha2::{
    Digest,
    Sha256,
};
use std::{
    env,
    fmt::Write,
    fs,
    path::Path,
    process::Command,
};

fn compute_sha256_8(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();

    let mut hex = String::with_capacity(8);
    for byte in result.iter().take(4) {
        let _ = write!(hex, "{:02x}", byte);
    }
    hex
}

fn run_git_command(args: &[&str]) -> Option<Vec<u8>> {
    let output = Command::new("git").args(args).output().ok()?;
    if output.status.success() {
        Some(output.stdout)
    } else {
        None
    }
}

fn is_inside_git_work_tree() -> bool {
    let Some(stdout) = run_git_command(&["rev-parse", "--is-inside-work-tree"]) else {
        return false;
    };
    let Ok(output) = std::str::from_utf8(&stdout) else {
        return false;
    };
    output.trim() == "true"
}

fn get_git_tag() -> Option<String> {
    let stdout = run_git_command(&["describe", "--tags", "--exact-match", "HEAD"])?;
    let tag = String::from_utf8(stdout).ok()?.trim().to_string();
    if tag.is_empty() { None } else { Some(tag) }
}

fn get_git_branch() -> Option<String> {
    if let Some(stdout) = run_git_command(&["branch", "--show-current"])
        && let Ok(branch) = String::from_utf8(stdout)
    {
        let trimmed = branch.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    if let Some(stdout) = run_git_command(&["rev-parse", "--abbrev-ref", "HEAD"])
        && let Ok(branch) = String::from_utf8(stdout)
    {
        let trimmed = branch.trim();
        if !trimmed.is_empty() && trimmed != "HEAD" {
            return Some(trimmed.to_string());
        }
    }

    None
}

fn get_git_diff_and_untracked() -> Option<(Vec<u8>, bool)> {
    let diff_stdout = run_git_command(&["diff", "--binary", "HEAD"])?;
    let untracked_stdout = run_git_command(&["ls-files", "--others", "--exclude-standard", "-z"])?;

    let mut combined_data = Vec::new();
    let mut is_modified = !diff_stdout.is_empty();

    combined_data.extend_from_slice(&diff_stdout);

    let untracked_paths: Vec<&[u8]> = untracked_stdout
        .split(|&b| b == 0)
        .filter(|p| !p.is_empty())
        .collect();

    if !untracked_paths.is_empty() {
        is_modified = true;
        let mut sorted_paths = untracked_paths;
        sorted_paths.sort();

        for path_bytes in sorted_paths {
            combined_data.extend_from_slice(path_bytes);
            if let Ok(path_str) = std::str::from_utf8(path_bytes) {
                let path = Path::new(path_str);
                if path.is_file()
                    && let Ok(content) = fs::read(path)
                {
                    combined_data.extend_from_slice(&content);
                }
            }
        }
    }

    Some((combined_data, is_modified))
}

fn main() {
    let pkg_version = env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.1.0".to_string());

    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/index");
    println!("cargo:rerun-if-changed=.git/refs");
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=build/version.rs");

    if !is_inside_git_work_tree() {
        let fallback_version = version::compute_fallback_version(&pkg_version);
        println!("cargo:rustc-env=AGW_VERSION={fallback_version}");
        return;
    }

    let tag = get_git_tag();
    let branch = get_git_branch();
    let (combined_data, is_modified) = match get_git_diff_and_untracked() {
        Some(res) => res,
        None => {
            let fallback_version = version::compute_fallback_version(&pkg_version);
            println!("cargo:rustc-env=AGW_VERSION={fallback_version}");
            return;
        },
    };

    let diff_hash = if is_modified {
        compute_sha256_8(&combined_data)
    } else {
        compute_sha256_8(&[])
    };

    let final_version = version::compute_version(tag.as_deref(), is_modified, branch.as_deref(), &diff_hash);

    println!("cargo:rustc-env=AGW_VERSION={final_version}");
}
