fn main() {
    // Link libmpv (installed via Homebrew)
    println!("cargo:rustc-link-lib=dylib=mpv");
    println!("cargo:rustc-link-search=native=/opt/homebrew/lib");

    // OpenGL.framework for the mpv render API
    println!("cargo:rustc-link-lib=framework=OpenGL");

    tauri_build::build()
}
