fn main() {
    cc::Build::new()
        .file("c_src/DXBCChecksum.c")
        .compile("DXBCChecksum");
}
