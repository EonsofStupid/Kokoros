// Platform-gated link configuration for kokoros.
//
// Linux  – probe sonic and pcaudio via pkg-config; fall back to standard
//           system paths if pkg-config has no entry for them.
// macOS  – probe via pkg-config (Homebrew).  pcaudio is disabled in the
//           espeak-ng CMake build on macOS so failures are non-fatal.
// Windows – espeak-ng is compiled from source by espeak-rs-sys (vendored CMake
//            build).  sonic and pcaudio are bundled within that build or are
//            not required on Windows.  Any additional vcpkg-managed deps can be
//            probed here.  ORT is loaded dynamically at runtime (load-dynamic)
//            so no static ORT link directives are emitted.

fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    match target_os.as_str() {
        "linux" => build_linux(),
        "macos" => build_macos(),
        "windows" => build_windows(),
        _ => {}
    }
}

fn build_linux() {
    // Probe sonic via pkg-config first.  If the package is not registered
    // (older distros ship libsonic-dev without a .pc file), fall back to
    // searching the standard system library directories.
    if pkg_config::probe_library("sonic").is_err() {
        println!("cargo:rustc-link-search=/usr/lib");
        println!("cargo:rustc-link-search=/usr/lib/x86_64-linux-gnu");
        println!("cargo:rustc-link-search=/usr/lib64");
        println!("cargo:rustc-link-lib=dylib=sonic");
    }

    // Probe pcaudio.  The pkg-config name used by libpcaudio-dev is
    // "libpcaudio"; fall back to the raw library name when not found.
    if pkg_config::probe_library("libpcaudio").is_err() {
        println!("cargo:rustc-link-lib=dylib=pcaudio");
    }
}

fn build_macos() {
    // On macOS, espeak-ng's CMake build sets USE_LIBPCAUDIO=OFF so pcaudio is
    // not required.  sonic may or may not be needed depending on the espeak-ng
    // version; probe it best-effort via pkg-config (Homebrew).
    let _ = pkg_config::probe_library("sonic");
    let _ = pkg_config::probe_library("libpcaudio");
}

fn build_windows() {
    // espeak-ng is compiled from source by espeak-rs-sys using its vendored
    // CMake build; no additional link directives for espeak-ng are needed here.
    //
    // Set ESPEAK_STATIC_CRT=1 (configured via .cargo/config.toml) so that the
    // espeak-rs-sys CMake build uses /MT (static CRT), matching the Rust binary.
    //
    // If your project has additional native dependencies managed through vcpkg,
    // probe them below.  Use the x64-windows-static triplet to ensure /MT
    // compatibility.  See docs/windows-msvc.md for setup instructions.
    //
    // Example:
    //   vcpkg::Config::new()
    //       .target_triplet("x64-windows-static")
    //       .find_package("some-extra-lib")
    //       .expect("Install with: vcpkg install some-extra-lib:x64-windows-static");
}

