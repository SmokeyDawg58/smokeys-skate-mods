//! The build's secret: see `../build_secret.rs`. A development build makes it anew whenever
//! anything the executable is built from changes. On Windows the executable also carries its
//! icon, `skate2.ico`, and the window thread's stack size.

#[path = "../build_secret.rs"]
mod build_secret;

#[path = "../test_build_metadata.rs"]
mod test_build_metadata;

fn main() {
    test_build_metadata::emit();
    embed_skate3_catch();
    embed_friend_rain();
    embed_local_pool_ground();
    embed_skate3_landing_notice();
    embed_smokey_updater();
    // Embedded-player loading also runs on the main thread in bounded live
    // diagnostics. Match the production simulation worker's 16 MiB reserve.
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        println!("cargo:rustc-link-arg-bin=skate2-pc=/STACK:16777216");
    }
    build_secret::emit(&["build.rs", "../build_secret.rs", "Cargo.toml", "src", "../../crates", "../../Cargo.toml", "../../Cargo.lock"]);
    let target = |key: &str, value: &str| std::env::var(key).is_ok_and(|actual| actual == value);
    if target("CARGO_CFG_TARGET_OS", "windows") && target("CARGO_CFG_TARGET_ENV", "msvc") {
        embed_icon("skate2.ico");
        // The window thread loads the menus and builds the client by value, and with
        // SKATE2_SERIAL_SIM it runs the simulation too: it gets the SIM thread's 16 MB, where
        // Windows reserves 1 MB (Linux and macOS 8).
        println!("cargo:rustc-link-arg-bins=/STACK:{}", 16 << 20);
    }
}


// The authored pool samples are derived from game data and stay outside Git.
fn embed_local_pool_ground() {
    println!("cargo:rerun-if-env-changed=SKATE2_POOL_GROUND");
    println!("cargo:rerun-if-env-changed=SKATE3_EMBEDDED_PACK");
    let bytes = match std::env::var_os("SKATE2_POOL_GROUND") {
        Some(path) => {
            println!("cargo:rerun-if-changed={}", std::path::Path::new(&path).display());
            let bytes = std::fs::read(path).expect("read local pool-ground build input");
            assert_eq!(bytes.len(), 7 * (8 + 73 * 73 * 4), "unexpected pool-ground input size");
            bytes
        }
        None => {
            assert!(std::env::var_os("SKATE3_EMBEDDED_PACK").is_none(),
                "Full Skate 3 mod builds need SKATE2_POOL_GROUND; see docs/BUILDING-SMOKEY.md");
            Vec::new()
        }
    };
    let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").expect("cargo output"));
    std::fs::write(out.join("pool-ground.bin"), bytes).expect("local pool-ground output");
}

/// External original-game input, like the embedded Skate 3 player pack.
fn embed_skate3_catch() {
    println!("cargo:rerun-if-env-changed=SKATE3_DARKCATCH_BANK");
    let bytes = if let Some(path) = std::env::var_os("SKATE3_DARKCATCH_BANK") {
        println!("cargo:rerun-if-changed={}", std::path::Path::new(&path).display());
        let bytes = std::fs::read(path).expect("original Skate 3 darkcatch bank");
        assert!(bytes.starts_with(b"SPLC"), "Skate 3 darkcatch input must be SPLC");
        bytes
    } else {
        assert!(std::env::var_os("SKATE3_EMBEDDED_PACK").is_none(), "Skate 3 mod builds require SKATE3_DARKCATCH_BANK");
        Vec::new()
    };
    let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").expect("cargo output"));
    std::fs::write(out.join("skate3-darkcatch.bnk"), bytes).expect("embedded Skate 3 catch output");
}

/// Links `icon` into the executable as icon group 1, the one Explorer shows and the window
/// loads: a resource file holding each of its images as an `RT_ICON` and the `RT_GROUP_ICON`
/// that lists them, which the MSVC linker takes as an input.
fn embed_icon(icon: &str) {
    println!("cargo:rerun-if-changed={icon}");
    let file = std::fs::read(icon).expect("the executable's icon");
    let word = |at: usize| u16::from_le_bytes([file[at], file[at + 1]]);
    let dword = |at: usize| {
        let value = u32::from_le_bytes([file[at], file[at + 1], file[at + 2], file[at + 3]]);
        usize::try_from(value).expect("an icon image inside the file")
    };
    let count = word(4);
    let mut resources = resource(0, 0, 0, &[]);
    let mut group = [0_u16, 1, count].iter().flat_map(|value| value.to_le_bytes()).collect::<Vec<_>>();
    for id in 1..=count {
        let entry = 6 + 16 * usize::from(id - 1);
        let (size, offset) = (dword(entry + 8), dword(entry + 12));
        resources.extend(resource(3, id, 0x1010, &file[offset..offset + size]));
        group.extend_from_slice(&file[entry..entry + 12]);
        group.extend(id.to_le_bytes());
    }
    resources.extend(resource(14, 1, 0x1030, &group));
    let path = std::path::Path::new(&std::env::var("OUT_DIR").expect("cargo's output directory")).join("skate2.res");
    std::fs::write(&path, resources).expect("the icon's resource file");
    println!("cargo:rustc-link-arg-bins={}", path.display());
}

/// One entry of a 32-bit resource file: its header, with the type and name as ordinals and the
/// neutral language, then its data padded to four bytes. Type 0 is the empty entry such a file
/// starts with.
fn resource(kind: u16, name: u16, flags: u16, data: &[u8]) -> Vec<u8> {
    let size = u32::try_from(data.len()).expect("a resource under 4 GB");
    let mut entry = Vec::with_capacity(32 + data.len() + 3);
    entry.extend(size.to_le_bytes());
    entry.extend(32_u32.to_le_bytes());
    entry.extend([0xffff, kind, 0xffff, name].iter().flat_map(|value| value.to_le_bytes()));
    entry.extend(0_u32.to_le_bytes());
    entry.extend(flags.to_le_bytes());
    entry.extend(0_u16.to_le_bytes());
    entry.extend([0_u8; 8]);
    entry.extend_from_slice(data);
    entry.resize(entry.len().next_multiple_of(4), 0);
    entry
}

/// The original HUD slice stays beside the external player pack, like the catch bank.
fn embed_skate3_landing_notice() {
    println!("cargo:rerun-if-env-changed=SKATE3_EMBEDDED_PACK");
    let bytes = if let Some(pack) = std::env::var_os("SKATE3_EMBEDDED_PACK") {
        let path = std::path::PathBuf::from(pack).with_file_name("skate3-landing-notifier.bin");
        println!("cargo:rerun-if-changed={}", path.display());
        let bytes = std::fs::read(path).expect("original Skate 3 landing notifier build input");
        assert!(bytes.starts_with(b"S3LN0001"), "Skate 3 landing notifier format");
        bytes
    } else { Vec::new() };
    let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").expect("cargo output"));
    std::fs::write(out.join("skate3-landing-notifier.bin"),bytes).expect("embedded landing notifier output");
}

// External user-provided rain loop, packetized with the supplied native converter.
fn embed_friend_rain(){
    let out=std::path::PathBuf::from(std::env::var_os("OUT_DIR").expect("cargo output"));
    for (key,name) in [("SKATE2_RAIN_HEADER","friend-rain-loop.snr"),("SKATE2_RAIN_STREAM","friend-rain-loop.sns")] {
        println!("cargo:rerun-if-env-changed={key}");
        let bytes=if let Some(path)=std::env::var_os(key){
            println!("cargo:rerun-if-changed={}",std::path::Path::new(&path).display());
            std::fs::read(path).expect("external user rain input")
        }else{Vec::new()};
        std::fs::write(out.join(name),bytes).expect("embedded friend rain output");
    }
}

/// Build the public-key-only helper with the Windows .NET Framework already
/// required by the existing updater. No publisher keys or tokens are embedded.
fn embed_smokey_updater() {
    let out=std::path::PathBuf::from(std::env::var_os("OUT_DIR").expect("cargo output"));
    let result=out.join("smokey-updater.exe");
    let dir=std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("../../tools/smokey-updater");
    let sources=["Core.cs","Trust.cs","ManualInstall.cs","UpdateFlow.cs","SafeRestore.cs","OfficialFallback.cs","FallbackInstall.cs","FallbackMode.cs","SmokeyUpdater.cs"];
    for name in sources.iter().chain(["FallbackLauncher.cs","smokey.ico","smokey-clean.png"].iter()) {
        println!("cargo:rerun-if-changed={}",dir.join(name).display());
    }
    println!("cargo:rerun-if-changed=skate2.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref()!=Ok("windows") || !cfg!(windows) {
        std::fs::write(result,[]).expect("non-Windows updater stub");return;
    }
    let windows=std::env::var_os("SystemRoot").unwrap_or_else(||r"C:\Windows".into());
    let framework=std::path::PathBuf::from(windows).join(r"Microsoft.NET\Framework64\v4.0.30319");
    let launcher=out.join("smokey-fallback-launcher.exe");
    let mut wrapper=std::process::Command::new(framework.join("csc.exe"));
    wrapper.args(["/nologo","/optimize+","/warn:4","/warnaserror+","/platform:x64","/target:winexe","/main:SkateShared.FallbackLauncher"])
        .arg(format!("/out:{}",launcher.display()))
        .arg(format!("/win32icon:{}",std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("skate2.ico").display()));
    for name in ["System.dll","System.Core.dll","System.Security.dll","System.Runtime.Serialization.dll","System.IO.Compression.dll","System.IO.Compression.FileSystem.dll","System.Windows.Forms.dll","System.Drawing.dll"] {
        wrapper.arg(format!("/reference:{}",framework.join(name).display()));
    }
    for name in ["Core.cs","ManualInstall.cs","UpdateFlow.cs","SafeRestore.cs","FallbackInstall.cs","FallbackLauncher.cs"] {wrapper.arg(dir.join(name));}
    let wrapped=wrapper.output().expect("Windows .NET Framework C# compiler for fallback launcher");
    assert!(wrapped.status.success(),"Fallback launcher compile failed: {}",String::from_utf8_lossy(&wrapped.stdout));
    let mut cmd=std::process::Command::new(framework.join("csc.exe"));
    cmd.args(["/nologo","/optimize+","/warn:4","/warnaserror+","/platform:x64","/target:winexe"])
        .arg(format!("/out:{}",result.display())).arg(format!("/win32icon:{}",dir.join("smokey.ico").display()))
        .arg(format!("/resource:{},smokey.wordmark",dir.join("smokey-clean.png").display()))
        .arg(format!("/resource:{},smokey.fallback_launcher",launcher.display()));
    for name in ["System.dll","System.Core.dll","System.Security.dll","System.Runtime.Serialization.dll","System.IO.Compression.dll","System.IO.Compression.FileSystem.dll","System.Windows.Forms.dll","System.Drawing.dll"] {
        cmd.arg(format!("/reference:{}",framework.join(name).display()));
    }
    for name in sources {cmd.arg(dir.join(name));}
    let output=cmd.output().expect("Windows .NET Framework C# compiler for Smokey updater");
    assert!(output.status.success(),"Smokey updater compile failed: {}",String::from_utf8_lossy(&output.stdout));
}
