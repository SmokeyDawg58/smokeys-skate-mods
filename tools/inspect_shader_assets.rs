//! Read-only original shader manifest census, reusing the production decoders.
#[path = "../crates/skate2-formats/src/big.rs"]
mod big;
#[path = "../crates/skate2-formats/src/refpack.rs"]
mod refpack;
#[path = "../crates/skate2-formats/src/shader.rs"]
mod shader;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).ok_or("render.big path required")?;
    let bytes = std::fs::read(path)?;
    let archive = big::BigArchive::parse(&bytes)?;
    // Optional exact filename extraction for inspecting original UPDB source.
    // Keep original asset bytes outside the repository.
    if let (Some(filename), Some(destination)) = (std::env::args().nth(2), std::env::args().nth(3))
    {
        for entry in &archive.entries {
            let big::BigEntryKey::Name(name) = &entry.key else {
                continue;
            };
            if !name.to_ascii_lowercase().contains("\\final\\")
                || !name
                    .rsplit('\\')
                    .next()
                    .is_some_and(|n| n.eq_ignore_ascii_case(&filename))
            {
                continue;
            }
            let raw = bytes
                .get(entry.data_range()?)
                .ok_or("entry outside archive")?;
            let decoded = if refpack::decompressed_size(raw).is_ok() {
                refpack::decompress(raw)?
            } else {
                raw.to_vec()
            };
            std::fs::write(destination, &decoded)?;
            println!("extracted {name}: {} bytes", decoded.len());
            return Ok(());
        }
        return Err(format!("final shader asset not found: {filename}").into());
    }
    for entry in &archive.entries {
        let big::BigEntryKey::Name(name) = &entry.key else {
            continue;
        };
        if !name.to_ascii_lowercase().ends_with(".xml") {
            continue;
        }
        let raw = bytes
            .get(entry.data_range()?)
            .ok_or("entry outside archive")?;
        let decoded = if refpack::decompressed_size(raw).is_ok() {
            refpack::decompress(raw)?
        } else {
            raw.to_vec()
        };
        let Ok(manifest) = shader::ShaderManifest::parse(&decoded) else {
            continue;
        };
        println!("MANIFEST {name}");
        for technique in &manifest.techniques {
            for pass in &technique.passes {
                println!(
                    "  technique={} pass={} states={:?}",
                    technique.name, pass.id, pass.states
                );
                for sampler in &pass.samplers {
                    println!("  sampler={} states={:?}", sampler.name, sampler.states);
                }
            }
        }
    }
    Ok(())
}
