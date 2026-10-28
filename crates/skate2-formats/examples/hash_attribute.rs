use skate2_formats::attrib::string_hash64;

fn main() {
    for value in std::env::args().skip(1) {
        println!("{value}: {:016x}", string_hash64(&value));
    }
}
