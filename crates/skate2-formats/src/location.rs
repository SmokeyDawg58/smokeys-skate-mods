//! Plain-text world location tables (`*_Sim.loc`).

use std::{fmt, str};

#[derive(Clone, Debug, PartialEq)]
pub struct Location {
    pub name: String,
    pub category: Option<String>,
    /// Row-major 4x4 transform as serialized by the reference tools.
    pub transform: [f32; 16],
}

impl Location {
    #[must_use]
    pub fn position(&self) -> [f32; 3] {
        [self.transform[12], self.transform[13], self.transform[14]]
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct LocationTable {
    pub locations: Vec<Location>,
}

impl LocationTable {
    /// Parses a reference `LocationTable` document.
    ///
    /// # Errors
    ///
    /// Returns [`LocationError`] for invalid UTF-8, missing fields, malformed
    /// transforms, or an unterminated location record.
    pub fn parse(bytes: &[u8]) -> Result<Self, LocationError> {
        let document = str::from_utf8(bytes).map_err(|_| LocationError::InvalidUtf8)?;
        if !document.contains("<LocationTable>") {
            return Err(LocationError::MissingRoot);
        }

        let mut locations = Vec::new();
        let mut remainder = document;
        while let Some(start) = remainder.find("<Location>") {
            remainder = &remainder[start + "<Location>".len()..];
            let end = remainder
                .find("</Location>")
                .ok_or(LocationError::UnterminatedLocation)?;
            let record = &remainder[..end];
            remainder = &remainder[end + "</Location>".len()..];

            let name = element(record, "Name")
                .ok_or(LocationError::MissingName)?
                .trim()
                .to_owned();
            let category = element(record, "Category")
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_owned);
            let transform_text = element(record, "Transform")
                .ok_or_else(|| LocationError::MissingTransform(name.clone()))?;
            let mut values = [0.0_f32; 16];
            let mut count = 0_usize;
            for component in transform_text.split(',') {
                if count == values.len() {
                    return Err(LocationError::TransformComponentCount {
                        name,
                        actual: count + 1,
                    });
                }
                values[count] = component.trim().parse().map_err(|_| {
                    LocationError::InvalidTransformComponent {
                        name: name.clone(),
                        component: component.trim().to_owned(),
                    }
                })?;
                count += 1;
            }
            if count != values.len() {
                return Err(LocationError::TransformComponentCount {
                    name,
                    actual: count,
                });
            }
            locations.push(Location {
                name,
                category,
                transform: values,
            });
        }
        Ok(Self { locations })
    }

    #[must_use]
    pub fn find(&self, name: &str) -> Option<&Location> {
        self.locations
            .iter()
            .find(|location| location.name.eq_ignore_ascii_case(name))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LocationError {
    InvalidUtf8,
    MissingRoot,
    UnterminatedLocation,
    MissingName,
    MissingTransform(String),
    TransformComponentCount { name: String, actual: usize },
    InvalidTransformComponent { name: String, component: String },
}

impl fmt::Display for LocationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid Skate 2 location table: {self:?}")
    }
}

impl std::error::Error for LocationError {}

fn element<'a>(record: &'a str, tag: &str) -> Option<&'a str> {
    let opening = format!("<{tag}>");
    let closing = format!("</{tag}>");
    let start = record.find(&opening)? + opening.len();
    let end = record[start..].find(&closing)? + start;
    Some(&record[start..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = r"<LocationTable>
    <Location>
        <Name>A_RobsPark</Name>
        <Category>DIST_Waterfront</Category>
        <Transform>1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 588.861694, -54.231647, 996.504822, 1</Transform>
    </Location>
    <Location>
        <Name>Uncategorized</Name>
        <Category />
        <Transform>1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 2, 3, 4, 1</Transform>
    </Location>
</LocationTable>";

    #[test]
    fn parses_named_locations_and_positions() {
        let table = LocationTable::parse(FIXTURE.as_bytes()).unwrap();
        let park = table.find("a_robspark").unwrap();

        assert_eq!(table.locations.len(), 2);
        assert_eq!(park.category.as_deref(), Some("DIST_Waterfront"));
        for (actual, expected) in
            park.position()
                .into_iter()
                .zip([588.861_7, -54.231_647, 996.504_8])
        {
            assert!((actual - expected).abs() < 0.000_1);
        }
        assert_eq!(table.locations[1].category, None);
    }

    #[test]
    fn rejects_incomplete_transforms() {
        let fixture = b"<LocationTable><Location><Name>x</Name><Transform>1, 2</Transform></Location></LocationTable>";
        assert!(matches!(
            LocationTable::parse(fixture),
            Err(LocationError::TransformComponentCount { actual: 2, .. })
        ));
    }
}
