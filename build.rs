fn main() {
    // Band 212: bundle winfsp.msi placeholder and set version env
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=installer/LinFS.iss");
    println!("cargo:rustc-env=LINFS_VERSION=1.0.0");
    println!("cargo:rustc-env=LINFS_BAND=212");
}
