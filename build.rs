use winresource::WindowsResource;

fn main() {
    cc::Build::new()
        .file("c_src/DXBCChecksum.c")
        .compile("DXBCChecksum");

    WindowsResource::new()
        .set_manifest_file("manifest.xml")
        .compile()
        .unwrap();
}
