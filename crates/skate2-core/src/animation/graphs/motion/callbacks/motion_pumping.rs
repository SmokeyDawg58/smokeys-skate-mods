//! Pumping1925 Update82c9ecc0; Begin is BLR, End82c9f2b0.
#[cfg(test)]
#[path = "motion_pumping_tests.rs"]
mod tests;
#[derive(Debug)]
pub(crate) struct PumpingInstance {
    pub(crate) influence: f32,
    pub(crate) smoothed: f32,
    pub(crate) channel: i32,
    pub(crate) peak: f32,
}
impl PumpingInstance {
    pub(crate) fn after_allocation(allocated: bool) -> Option<Self> {
        allocated.then_some(Self {
            influence: 0.0,
            smoothed: 0.0,
            channel: -1,
            peak: 0.0,
        })
    }
}
/// `NewChannel` source pack: priority0, cycle/mirrorfalse, speed1,
/// first-framefalse, last-frametrue, useAttributesfalse; only lengths are live.
#[derive(Clone, Copy, Debug)]
pub(crate) struct PumpingChannel {
    pub(crate) index: usize,
    pub(crate) blend_in: f32,
    pub(crate) blend_out: f32,
}
pub(crate) trait PumpingSource {
    type Error;
    fn resolve(&mut self, update: bool) -> Result<(), Self::Error>;
    fn allowed(&mut self) -> Result<bool, Self::Error>;
    fn missing_instance(&mut self) -> Self::Error;
    fn ground_acceleration(&mut self) -> Result<f32, Self::Error>;
    fn scalar(&mut self, key: u64) -> Result<f32, Self::Error>;
    fn graph(&mut self, input: f32) -> Result<f32, Self::Error>;
    fn has_channel(&mut self, index: usize) -> Result<bool, Self::Error>;
    fn enabled(&mut self) -> Result<bool, Self::Error>;
    fn new_channel(&mut self, request: PumpingChannel) -> Result<(), Self::Error>;
    fn influence(&mut self, index: usize, value: f32) -> Result<(), Self::Error>;
    fn end_channel(&mut self, index: usize) -> Result<(), Self::Error>;
}
fn select(test: f32, yes: f32, no: f32) -> f32 {
    if test >= 0.0 { yes } else { no }
}
#[allow(clippy::neg_cmp_op_on_partial_ord)] // Native branches complement individual CR bits.
pub(crate) fn update<S: PumpingSource>(
    s: &mut S,
    instance: Option<&mut PumpingInstance>,
) -> Result<(), S::Error> {
    s.resolve(true)?;
    if !s.allowed()? {
        return Ok(());
    }
    let v = instance.ok_or_else(|| s.missing_instance())?;
    let previous = v.influence;
    let acceleration = s.ground_acceleration()?;
    let scaled = acceleration / s.scalar(0x57bc_b525_1cc1_ed5c)?;
    let scaled = select(-scaled, 0.0, scaled);
    let scaled = select(1.0 - scaled, scaled, 1.0);
    let blend = s.scalar(0xc7e4_895c_512a_8550)?;
    v.smoothed = (1.0 - blend).mul_add(v.smoothed, blend * scaled);
    v.influence = s.graph(v.smoothed)?;
    let threshold = s.scalar(0xc47d_82b1_fb49_e9d7)?;
    // f07c orderedGT rejects previous; f08c requires current orderedGT.
    if !(previous > threshold) && v.influence > threshold {
        for index in 0..5 {
            if s.has_channel(index)? {
                continue;
            }
            let blend_in = s.scalar(0x7e08_4051_45cc_275d)?;
            let blend_out = s.scalar(0x0ff8_3496_1814_e5f9)?;
            if s.enabled()? {
                s.new_channel(PumpingChannel {
                    index,
                    blend_in,
                    blend_out,
                })?;
            }
            // Return status of NewChannel is ignored by source; even disabled
            // cvar still stores selected index and peak before influence gate.
            v.channel = i32::try_from(index).expect("native five pump channels");
            v.peak = v.influence;
            break;
        }
    }
    // f234/f248 BO4 BI24 = !LT including UN, NOT ordered >=.
    if v.channel >= 0 && !(v.influence < previous) && !(v.influence < v.peak) {
        v.peak = v.influence;
        s.influence(
            usize::try_from(v.channel).expect("checked nonnegative pump index"),
            v.influence,
        )?;
    }
    Ok(())
}
pub(crate) fn end<S: PumpingSource>(s: &mut S) -> Result<(), S::Error> {
    s.resolve(false)?;
    for index in 0..5 {
        s.end_channel(index)?;
    }
    Ok(())
}
