// Build script for screen-capture crate
// Compiles Objective-C bridge on macOS

use std::env;
use std::path::PathBuf;

fn main() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    
    if target_os == "macos" {
        println!("cargo:rerun-if-changed=src/macos/SCRecorder.m");
        println!("cargo:rerun-if-changed=src/macos/SCRecorder.h");
        
        // Compile the Objective-C bridge
        cc::Build::new()
            .file("src/macos/SCRecorder.m")
            .flag("-fobjc-arc") // Enable ARC
            .flag("-fmodules") // Enable modules
            .compile("SCRecorder");
        
        // Link frameworks
        println!("cargo:rustc-link-lib=framework=Foundation");
        println!("cargo:rustc-link-lib=framework=AVFoundation");
        println!("cargo:rustc-link-lib=framework=CoreMedia");
        println!("cargo:rustc-link-lib=framework=CoreVideo");
        println!("cargo:rustc-link-lib=framework=ScreenCaptureKit");
        println!("cargo:rustc-link-lib=framework=QuartzCore");
    }
}
