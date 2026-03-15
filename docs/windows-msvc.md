# Windows MSVC Build Guide

This document is the **single source of truth** for building and packaging Kokoros
on Windows with the MSVC toolchain.  It covers static CRT linkage, ONNX Runtime
dynamic loading, native dependency setup, and Tauri integration.

---

## Table of Contents

1. [Toolchain Requirements](#1-toolchain-requirements)
2. [Static CRT Policy (`/MT`)](#2-static-crt-policy-mt)
3. [ONNX Runtime: Dynamic Loading](#3-onnx-runtime-dynamic-loading)
4. [Native Dependencies via vcpkg](#4-native-dependencies-via-vcpkg)
5. [Building](#5-building)
6. [Tauri Integration and Packaging](#6-tauri-integration-and-packaging)
7. [Runtime DLL Lookup](#7-runtime-dll-lookup)
8. [Verifying the Build](#8-verifying-the-build)
9. [CI / Automated Builds](#9-ci--automated-builds)

---

## 1. Toolchain Requirements

| Requirement | Minimum version |
|---|---|
| Rust (stable) | 1.88 |
| Target | `x86_64-pc-windows-msvc` |
| MSVC Build Tools | Visual Studio 2019 / 2022 (C++ workload) |
| CMake | 3.16+ (for `espeak-rs-sys` vendored build) |
| clang / libclang | Required by `bindgen` inside `espeak-rs-sys` |
| vcpkg | Latest (optional – for extra native libs) |

Install the Rust target:

```powershell
rustup target add x86_64-pc-windows-msvc
```

Install clang (needed by `bindgen`):

```powershell
winget install LLVM.LLVM
# or via chocolatey:
choco install llvm
```

---

## 2. Static CRT Policy (`/MT`)

All Rust code and every C/C++ component compiled during the build **must** use
the static CRT (`/MT`) so the final binary carries no dependency on
`VCRUNTIME140.dll` / `MSVCP140.dll`.

### Rust code

`.cargo/config.toml` already enforces this for the `x86_64-pc-windows-msvc`
target:

```toml
[target.x86_64-pc-windows-msvc]
rustflags = ["-C", "target-feature=+crt-static"]
```

### espeak-ng (vendored CMake build)

The `espeak-rs-sys` crate builds espeak-ng from its bundled source using CMake.
The build script reads the `ESPEAK_STATIC_CRT` environment variable to decide
whether to pass `/MT` flags to the CMake build.

`.cargo/config.toml` sets this automatically for **all** platforms
(harmless on Linux/macOS where the MSVC flag is never applied):

```toml
[env]
ESPEAK_STATIC_CRT = "1"
```

If you need to override this per-run:

```powershell
$env:ESPEAK_STATIC_CRT = "1"
cargo build --release --target x86_64-pc-windows-msvc
```

### vcpkg-managed libraries (optional extra deps)

When probing native libraries through vcpkg, use the **static** triplet to
ensure `/MT` compatibility:

```
x64-windows-static
```

---

## 3. ONNX Runtime: Dynamic Loading

On Windows, Kokoros uses `ort`'s `load-dynamic` feature.  This means:

- **No static ORT linkage** – `onnxruntime.lib` is never linked at build time.
- **No CRT conflict** – the Rust binary stays fully `/MT`; the ORT DLL brings
  its own CRT copy.
- **Runtime discovery** – `onnxruntime.dll` is located and loaded by
  `kokoros/src/onn/ort_runtime.rs` before the first ONNX session is created.

### Why `load-dynamic`?

Statically linking a prebuilt `onnxruntime.lib` into a `/MT` binary almost
always causes a CRT conflict because the prebuilt ORT libraries are commonly
shipped as `/MD` (dynamic CRT).  Dynamic loading sidesteps this entirely.

### What ORT version does the DLL need to be?

The `ort` crate version in `kokoros/Cargo.toml` is `2.0.0-rc.11`, which targets
**ONNX Runtime 1.23.x**.  Use an ORT 1.23.x release DLL.

Download from the official ONNX Runtime releases:
<https://github.com/microsoft/onnxruntime/releases>

---

## 4. Native Dependencies via vcpkg

### What is already handled automatically

| Library | How it is resolved |
|---|---|
| `espeak-ng` | **Vendored**: built from source by `espeak-rs-sys` via CMake.  No vcpkg needed. |
| `sonic` | Bundled within the espeak-ng CMake build on Windows. |
| `pcaudio` | Not used on Windows (disabled by `USE_LIBPCAUDIO=OFF` in the CMake build). |
| `opus` | Handled by the `opus` crate's own build script (uses vcpkg when available). |

### Setting up vcpkg (for optional extra deps)

Only required if you add new native C/C++ dependencies that are not bundled:

```powershell
git clone https://github.com/microsoft/vcpkg.git C:\vcpkg
C:\vcpkg\bootstrap-vcpkg.bat
$env:VCPKG_ROOT = "C:\vcpkg"
```

Always use the static triplet to match `/MT`:

```powershell
vcpkg install <package-name>:x64-windows-static
```

Set the default triplet so you don't have to specify it each time:

```powershell
$env:VCPKG_DEFAULT_TRIPLET = "x64-windows-static"
```

---

## 5. Building

```powershell
# Ensure libclang is findable (adjust path as needed):
$env:LIBCLANG_PATH = "C:\Program Files\LLVM\bin"

# Build in release mode for Windows MSVC:
cargo build --release --target x86_64-pc-windows-msvc
```

The resulting binary is at:

```
target\x86_64-pc-windows-msvc\release\koko.exe
```

---

## 6. Tauri Integration and Packaging

### Where to place `onnxruntime.dll`

The engine searches for `onnxruntime.dll` in two locations (in order):

| Priority | Path | Use case |
|---|---|---|
| 1 (preferred) | `<exe_dir>/onnxruntime.dll` | DLL next to the exe |
| 2 (fallback) | `<exe_dir>/runtime/onnxruntime.dll` | Organised layout |

The **Tauri platform** (your desktop app) is the **source of truth** for where
the DLL is placed at runtime.  The engine only defines *where it looks*.

### Development workflow

```powershell
# Copy the ORT DLL next to the development binary:
Copy-Item onnxruntime.dll target\x86_64-pc-windows-msvc\release\

# Or use the organised layout:
New-Item -ItemType Directory target\x86_64-pc-windows-msvc\release\runtime -Force
Copy-Item onnxruntime.dll target\x86_64-pc-windows-msvc\release\runtime\
```

### Tauri bundler configuration

In `tauri.conf.json`, include the DLL as a resource so Tauri copies it
alongside the packaged executable:

```json
{
  "tauri": {
    "bundle": {
      "resources": [
        "onnxruntime.dll"
      ]
    }
  }
}
```

Or, using the organised `runtime/` layout:

```json
{
  "tauri": {
    "bundle": {
      "resources": {
        "onnxruntime.dll": "runtime/onnxruntime.dll"
      }
    }
  }
}
```

This places the DLL at `<exe_dir>/runtime/onnxruntime.dll`, which matches
fallback location (2) in the engine's search order.

---

## 7. Runtime DLL Lookup

The lookup is implemented in `kokoros/src/onn/ort_runtime.rs` and runs once
before the first ONNX session is created.

```
1. <exe_dir>/onnxruntime.dll          ← preferred (DLL next to app)
2. <exe_dir>/runtime/onnxruntime.dll  ← Tauri "organised" layout
3. (PATH – last resort via OS loader if neither file exists)
```

If neither location contains the DLL the engine returns a clear error:

```
onnxruntime.dll not found.

Checked locations:
  1. C:\Users\...\release\onnxruntime.dll
  2. C:\Users\...\release\runtime\onnxruntime.dll

Place onnxruntime.dll next to the executable (location 1) or in the
runtime/ subfolder (location 2) before launching the application.

See docs/windows-msvc.md for packaging instructions …
```

---

## 8. Verifying the Build

### Check that the binary uses the static CRT

```powershell
dumpbin /dependents target\x86_64-pc-windows-msvc\release\koko.exe
```

The output should **not** contain `VCRUNTIME140.dll` or `MSVCP140.dll`.

### Confirm ORT is not statically linked

```powershell
dumpbin /imports target\x86_64-pc-windows-msvc\release\koko.exe | Select-String "onnxruntime"
```

No ORT imports should appear – the library is loaded dynamically at runtime.

### Smoke test

```powershell
# Run without the DLL → should print a clear error message:
koko.exe --help

# Run with the DLL present → should initialise ORT successfully:
Copy-Item onnxruntime.dll .\
koko.exe -m checkpoints\kokoro-v1.0.onnx -d data\voices-v1.0.bin "Hello world"
```

---

## 9. CI / Automated Builds

See `.github/workflows/cross-platform.yml` for the GitHub Actions workflow that
builds and validates Kokoros on Windows, Linux, and macOS on every push and
pull request.

The Windows CI job:

- Uses the `x86_64-pc-windows-msvc` target.
- Installs LLVM/clang for `bindgen`.
- Sets `ESPEAK_STATIC_CRT=1` (already in `.cargo/config.toml`).
- Does **not** require `onnxruntime.dll` to be present – the build itself
  succeeds without the DLL (it is only needed at runtime).
