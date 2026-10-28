//! Pegasus tIrradianceData; Fixup829c7bb0 and PDB layout.
pub const TYPE_ID: u32 = 0x00eb_0024;
pub type Coefficients = [[f32; 4]; 9];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LightProbe {
    pub coefficients: Coefficients,
    pub position: [f32; 4],
}

/// The pointer is relative to the object's beginning, not the pointer field.
pub fn parse(bytes: &[u8]) -> Result<Vec<LightProbe>, &'static str> {
    let header = bytes.get(..8).ok_or("irradiance header truncated")?;
    let count = u32::from_be_bytes(header[..4].try_into().unwrap()) as usize;
    let offset = u32::from_be_bytes(header[4..].try_into().unwrap()) as usize;
    if count == 0 { return Ok(Vec::new()); }
    if offset < 8 { return Err("irradiance nonempty array has invalid pointer"); }
    let end = count.checked_mul(160).and_then(|n|offset.checked_add(n))
        .ok_or("irradiance array overflow")?;
    let data = bytes.get(offset..end).ok_or("irradiance array truncated")?;
    Ok(data.chunks_exact(160).map(|b| {
        let rows: [[f32; 4]; 10] = std::array::from_fn(|r| std::array::from_fn(|c| {
            let p = r * 16 + c * 4;
            f32::from_be_bytes(b[p..p+4].try_into().unwrap())
        }));
        LightProbe { coefficients: rows[..9].try_into().unwrap(), position: rows[9] }
    }).collect())
}

#[cfg(test)]
mod tests {
    #[test]
    fn relative_pointer_and_full_probe_stride() {
        let mut b = vec![0u8; 32 + 320];
        b[..4].copy_from_slice(&2u32.to_be_bytes());
        b[4..8].copy_from_slice(&32u32.to_be_bytes());
        b[32+144..32+148].copy_from_slice(&12f32.to_be_bytes());
        b[192..196].copy_from_slice(&3f32.to_be_bytes());
        let p=super::parse(&b).unwrap();
        assert_eq!(p[0].position[0],12.);
        assert_eq!(p[1].coefficients[0][0],3.);
        assert!(super::parse(&b[..b.len()-1]).is_err());
    }
}
