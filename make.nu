#!/usr/bin/env nu
#-*- nushell-ts -*-

use std/log

# Utils

let bin_home = ($env.XDG_BIN_HOME | default $"($env.HOME)/.local/bin")
let tmpdir = ($env.TMPDIR | default $"/tmp")

let build_dir = "./target/release"

let bin_name = "agwaita"

def commands [] {
  [ "install" "build" "hotrun" "run" "check" "clean" "init" ]
}

def commands_check [] {
  [ "std" "clippy" ]
}

def agwaita_log_level [] {
  ["CRITICAL" "ERROR" "WARNING" "INFO" "DEBUG"]
}

def prepare-hook [
  hook_name: string
  target_scripts: record<POSIX: string, NUSHELL: string>
  script_shell: string
] {
  let hook_path = $".git/hooks/($hook_name)"

  for target_script in ($target_scripts | values) {
    if not ($target_script | path exists) {
      error make { msg: $"script not found: ($target_script)" }
    }
  }

  if ($hook_path | path exists) {
    let backup_file_name = $"($hook_path).(date now | format date %s).bak"
    log info $"hook found, make backup: ($backup_file_name)"
    mv $hook_path $backup_file_name
  }

  let hook_content = $"#!/usr/bin/env sh\nexec ($target_scripts | get $script_shell)\n"

  log info $"creating hook: ($hook_path)..."
  $hook_content | save $hook_path
  chmod +x $hook_path
}


# Script

# Build and install `agwaita`
def "main install" [] {
  main init
  main check

  log info "build..."
  cargo build --workspace --all-targets --all-features --release
  let target = $"($bin_home)/($bin_name)"
  if ($target | path exists) {
    log warning "remove old binary before..."
    rm -f $target
  }
  log info "install..."
  cp -iu $"($build_dir)/($bin_name)" $bin_home
}

# Build `agwaita` into `build/` directory
def "main build" [] {
  log info "build..."
  cargo build --workspace --all-targets --all-features
}

# Run `agwaita` directly
def --wrapped "main hotrun" [
  --log-level(-l): string@agwaita_log_level = "DEBUG"
  ...args: string
] {
  with-env {AGWAITA_LOG_LEVEL: $log_level} {
    log info "run with cargo directly..."
    cargo run --package agw-cli --bin agwaita -- ...$args
  }
}

# Build and run `agwaita` binary
def "main run" [
  ...args: string
] {
  main build
  log info "run..."

  ./target/debug/agwaita ...$args
}

# Check all
def "main check" []: [nothing -> nothing] {
  log info "check..."
  main check fmt
  main check std
  main check clippy
}

# Check format with cargo
def "main check fmt" []: [nothing -> nothing] {
  log info "check fmt with cargo..."
  cargo +nightly fmt --check
}

# Check with cargo
def "main check std" []: [nothing -> nothing] {
  log info "check with cargo..."
  cargo check --workspace --all-targets --all-features
}

# Check with clippy
def "main check clippy" []: [nothing -> nothing] {
  log info "check with clippy..."
  cargo clippy --no-deps --all-targets -- -D warnings
}

## FIXME: not ready yet
# Check with cargo deny
def "main check deny" [--init]: [nothing -> nothing] {
  if $init {
  log info "init cargo deny..."
    cargo deny init
  }
  log info "check with cargo deny..."
  cargo deny check
}

# Clean workspace
def "main clean" [] {
  log info "clean..."
  cargo clean
}

# Initialize workspace
def "main init" [
  --hooks-posix-only # install hooks only for POSIX systems
] {
  log info "init workspace..."
  cargo fetch

  prepare-hook "pre-commit" {POSIX: "./scripts/pre-commit.sh" NUSHELL: "./scripts/pre-commit.nu"} (if $hooks_posix_only {"POSIX"} else {"NUSHELL"})
  prepare-hook "pre-push" {POSIX: "./scripts/pre-push.sh" NUSHELL: "./scripts/pre-push.nu"} (if $hooks_posix_only {"POSIX"} else {"NUSHELL"})
}

# Make script for agwaita
def main [
  command: string@commands
]: [nothing -> nothing] {}
