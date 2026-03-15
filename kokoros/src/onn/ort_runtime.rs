//! ONNX Runtime initialisation helper.
//!
//! On **Windows** (where `ort` is compiled with the `load-dynamic` feature)
//! the ORT shared library must be located and loaded *before* any
//! [`ort::session::builder::SessionBuilder`] is created.
//!
//! ## DLL search order (Windows)
//! 1. `<exe_dir>/onnxruntime.dll`  – preferred; place the DLL next to the app.
//! 2. `<exe_dir>/runtime/onnxruntime.dll`  – organised layout for Tauri bundles.
//! 3. PATH  – last resort; `ort` falls back to the OS loader when no explicit
//!    path is provided.
//!
//! ## Non-Windows
//! This function is a no-op; ORT initialises automatically via static or
//! system-dynamic linkage as configured by the build system.
//!
//! See `docs/windows-msvc.md` for packaging instructions.

/// Initialise ONNX Runtime.
///
/// Must be called once before the first [`ort::session::builder::SessionBuilder`]
/// is created.  Calling it multiple times is safe – subsequent calls are ignored
/// by the ORT environment singleton.
///
/// # Errors
/// Returns an error string on Windows if `onnxruntime.dll` cannot be found or
/// fails to load.  On other platforms this always returns `Ok(())`.
pub fn init_ort() -> Result<(), String> {
    #[cfg(windows)]
    {
        init_ort_windows()
    }
    #[cfg(not(windows))]
    {
        Ok(())
    }
}

/// Windows-specific ORT initialisation.
///
/// Searches for `onnxruntime.dll` in the two canonical locations and hands the
/// resolved path to `ort::init_from`, which uses `libloading` to open the
/// library without requiring it to be on PATH.
#[cfg(windows)]
fn init_ort_windows() -> Result<(), String> {
    use std::path::PathBuf;

    let exe_dir: PathBuf = std::env::current_exe()
        .map_err(|e| format!("Failed to determine executable path: {}", e))?
        .parent()
        .ok_or_else(|| "Cannot determine executable directory".to_string())?
        .to_path_buf();

    // 1) <exe_dir>/onnxruntime.dll
    let primary = exe_dir.join("onnxruntime.dll");
    // 2) <exe_dir>/runtime/onnxruntime.dll
    let fallback = exe_dir.join("runtime").join("onnxruntime.dll");

    let dll_path = if primary.exists() {
        primary
    } else if fallback.exists() {
        fallback
    } else {
        return Err(format!(
            "onnxruntime.dll not found.\n\
             \n\
             Checked locations:\n\
             \x20 1. {}\n\
             \x20 2. {}\n\
             \n\
             Place onnxruntime.dll next to the executable (location 1) or in the\n\
             runtime/ subfolder (location 2) before launching the application.\n\
             \n\
             See docs/windows-msvc.md for packaging instructions and the Tauri\n\
             bundler configuration that automates DLL placement.",
            primary.display(),
            fallback.display(),
        ));
    };

    ort::init_from(&dll_path)
        .map_err(|e| {
            format!(
                "Failed to load onnxruntime.dll from '{}'.\n\
                 Error: {}\n\
                 \n\
                 Ensure that:\n\
                 \x20 • The DLL architecture matches (x64 for x86_64-pc-windows-msvc).\n\
                 \x20 • The ORT version matches the version this crate was compiled against\n\
                 \x20   (ort = 2.0.0-rc.11 expects ONNX Runtime 1.23.x).\n\
                 \x20 • All DLL dependencies (e.g. CUDA libraries) are also on PATH.",
                dll_path.display(),
                e
            )
        })?
        .commit();

    Ok(())
}
