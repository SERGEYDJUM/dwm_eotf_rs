use winresource::WindowsResource;

fn main() {
    WindowsResource::new()
        .set_manifest_file("manifest.xml")
        .set_icon("icons/app.ico")
        .compile()
        .unwrap();
}
