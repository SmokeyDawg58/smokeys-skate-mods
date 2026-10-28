//! `Sk8::Xml::Boundary`: an invisible boundary's polygon (`data\boundary\<name>.xml`), its
//! `Point`s in order as x, z.

use quick_xml::{Reader, events::Event};

/// The polygon's points (`PointField`).
///
/// # Errors
/// A malformed document or point.
pub fn parse(bytes: &[u8]) -> Result<Vec<[f32; 2]>, &'static str> {
    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().trim_text(true);
    reader.config_mut().expand_empty_elements = true;
    let mut open = false;
    let mut root_seen = false;
    let mut points = Vec::new();
    loop {
        match reader.read_event().map_err(|_| "malformed Boundary XML")? {
            Event::Start(element) => {
                let name = element.name();
                match (open, name.as_ref()) {
                    (false, b"Boundary") if !root_seen => {
                        root_seen = true;
                        open = true;
                    }
                    (true, b"Point") => {
                        let text = reader.read_text(name).map_err(|_| "invalid Boundary point")?;
                        let text = std::str::from_utf8(text.as_ref()).map_err(|_| "invalid Boundary encoding")?;
                        let (x, z) = text.split_once(',').ok_or("invalid Boundary point")?;
                        let point = [
                            x.trim().parse::<f32>().map_err(|_| "invalid Boundary X")?,
                            z.trim().parse::<f32>().map_err(|_| "invalid Boundary Z")?,
                        ];
                        if !point.iter().all(|value| value.is_finite()) {
                            return Err("nonfinite Boundary point");
                        }
                        points.push(point);
                    }
                    _ => return Err("unsupported Boundary element"),
                }
            }
            Event::End(element) => {
                if !open || element.name().as_ref() != b"Boundary" {
                    return Err("unbalanced Boundary XML");
                }
                open = false;
            }
            Event::Eof => break,
            Event::Decl(_) | Event::Comment(_) => {}
            Event::Text(text) if text.as_ref().iter().all(u8::is_ascii_whitespace) => {}
            _ => return Err("unsupported Boundary content"),
        }
    }
    if !root_seen || open {
        return Err("incomplete Boundary XML");
    }
    Ok(points)
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn a_boundary_reads_its_points_in_order() {
        let xml = br#"<?xml version="1.0" encoding="us-ascii"?>
<Boundary>
  <Point>555.8949, 248.7957</Point>
  <Point>691.9917, 254.9644</Point>
  <Point>691, 786</Point>
  <Point>541, 786</Point>
</Boundary>"#;
        assert_eq!(parse(xml).unwrap(), vec![[555.8949, 248.7957], [691.9917, 254.9644], [691.0, 786.0], [541.0, 786.0]]);
    }

    #[test]
    fn a_broken_boundary_is_an_error() {
        assert!(parse(b"<Boundary><Point>1, 2</Point>").is_err());
        assert!(parse(b"<Boundary><Point>1 2</Point></Boundary>").is_err());
        assert!(parse(b"<Boundary><Vertex>1, 2</Vertex></Boundary>").is_err());
        assert!(parse(b"<Points/>").is_err());
        assert_eq!(parse(b"<Boundary/>").unwrap(), Vec::<[f32; 2]>::new());
    }
}
