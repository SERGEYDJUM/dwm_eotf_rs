use winresource::WindowsResource;

fn main() {
    WindowsResource::new()
        .set_manifest_file("manifest.xml")
        .compile()
        .unwrap();
}
