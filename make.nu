#!/usr/bin/env nu
#-*- nushell-ts -*-

use std/log

# Utils

let bin_home = ($env.XDG_BIN_HOME | default $"($env.HOME)/.local/bin")
let tmpdir = ($env.TMPDIR | default $"/tmp")


let build_dir = "build"

let at_girs_dir = "@girs"

let tmpdir_ags = $"($tmpdir)/ags"
let hash_types_file = "types.hash"
let hash_types_path = $"($build_dir)/types.hash"
let hash_types_tmp_path = $"($tmpdir_ags)/types.hash"

let bin_name = "ags-shell"

def "sha1sum_all" [path: path]: [nothing -> string] {
  let hash = (fd . $path -x sha1sum | lines | sort | into string)
  return $hash
}

def commands [] {
  [ "install" "build" "hotrun" "run" "check" "clean" "init" ]
}

def commands_check [] {
  [ "install" "build" "hotrun" "run" "check" "clean" "init" ]
}

# Script

# Build and install `ags-shell`
def "main install" [] {
  main build
  log info "install..."
  cp $"($build_dir)/($bin_name)" $bin_home
}

# Build `ags-shell` into `build/` directory
def "main build" [] {
  main init
  main check style
  main check ts lint
  log info "build..."
  mkdir $build_dir
  ags bundle -g 4 app.tsx $bin_name
  chmod +x $bin_name
  mv $bin_name $"($build_dir)/($bin_name)"
}

# Run `ags-shell` directly
def "main hotrun" [] {
  main init
  log info "run with ags directly..."
  ags run ./app.tsx
}

# Build and run `ags-shell` binary
def "main run" [] {
  main build
  log info "run..."
  nu -c $"./($build_dir)/($bin_name)"
}

# Check all
def "main check" [
  command: string@commands_check
]: [nothing -> nothing] {
  main check types
  main check ts lint
  main check style
}

# Check TypeScript types
def "main check types" []: [nothing -> bool] {
  log info "check TypeScript types..."
  mkdir $"($tmpdir_ags)/"

  mut result = true
  if ("@girs" | path exists) == false {
    log warning "@girs directory not exists"
    $result = false
  }
  if ($hash_types_path | path exists) == false {
    log warning $"($hash_types_path) file not exist"
    $result =  false
  }
  if ("build" | path exists) == false {
    log warning "build directory not exist"
    $result =  false
  }
  if $result {
    let hash = (sha1sum_all @girs)
    $hash | save $"($hash_types_tmp_path)" --force
    if (cat $hash_types_tmp_path) != (cat $hash_types_path) {
      log warning "types are not up to date"
      print $"diff --recursive --color=auto ($hash_types_path) ($"($hash_types_tmp_path)")"
      try {
        diff --recursive --color=auto $hash_types_path $"($hash_types_tmp_path)"
      }
      $result = false
    }
  } else {
    log error "check types failed"
  }
  return $result
}

# Check TypeScript lint
def "main check ts lint" [] {
  log info "check TypeScript lint..."
  npx tsc --noEmit
}

# Check code style
def "main check style" [] {
  log info "check style..."
  npx eslint . --ext .ts,.tsx
}

# Clean workspace
def "main clean" [] {
  log info "clean..."
  if ("@girs" | path exists) {
    log debug $"remove ($at_girs_dir) directory"
    rm -rf $at_girs_dir
  }
  if ("build" | path exists) {
    log debug $"remove ($build_dir) directory"
    rm -rf $build_dir
  }
}

# Initialize workspace
def "main init" [] {
  if (main check types) == false {
    log info "init workspace..."
    mkdir $build_dir
    touch $hash_types_path
    ags types
    let gi_content = (
      open $"($at_girs_dir)/gi.d.ts"
      | lines
      | where {|l| $l | str starts-with "import" }
      | sort
      | insert 0 "/**\n * This file exports all GIR module type definitions.\n */\n\n"
      | str join "\n"
    )
    $"($gi_content)\n" | save --force $"($at_girs_dir)/gi.d.ts"
    sha1sum_all $at_girs_dir | save $"($hash_types_path)" --force
  }
  npm install
}

# Make script for ags-shell
def main [
  command: string@commands
]: [nothing -> nothing] {}
