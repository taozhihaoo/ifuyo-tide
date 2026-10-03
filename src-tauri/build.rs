use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    tauri_build::build();

    // Integration-test executables link the tao/comctl32 windowing stack
    // (the download engine emits Tauri events). Without a Common-Controls
    // 6 side-by-side manifest they load ancient comctl32 5.82 and die with
    // STATUS_ENTRYPOINT_NOT_FOUND at process start. The tauri-build
    // manifest only covers the main binary, and embedding a second
    // manifest resource there would collide (CVT1100) — so tests get the
    // manifest via a link-arg input, which merges with rustc's default.
    let manifest = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <dependency>
    <dependentAssembly>
      <assemblyIdentity type="win32" name="Microsoft.Windows.Common-Controls" version="6.0.0.0" processorArchitecture="*" publicKeyToken="6595b64144ccf1df" language="*"/>
    </dependentAssembly>
  </dependency>
</assembly>
"#;
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let manifest_path = out_dir.join("integration-tests.manifest");
    fs::write(&manifest_path, manifest).expect("write test manifest");
    println!("cargo:rustc-link-arg-tests=/MANIFEST:EMBED");
    println!(
        "cargo:rustc-link-arg-tests=/MANIFESTINPUT:{}",
        manifest_path.display()
    );
}
