//! `Recognizer::Matcher` 826e0b58/1038/1408 and retained history capability.
//! Call-count time is native; no wall clock or synthetic delta time is sampled.
#![cfg_attr(
    not(test),
    expect(dead_code, reason = "retained gameplay recognizer integration pending")
)]
use crate::{
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeInstance, AttributeRuntimeVault},
    input_pattern::{InputPattern, InputPatterns},
};
use std::{cmp::Ordering, collections::VecDeque, fmt, sync::Arc};

/// An actual selected history owner, not a copied array. The producer must
/// supply source post-deadzone points; `PlayerInput` scheduling is separate.
pub(crate) struct InputPatternHistory {
    occurrence: Arc<()>,
    points: VecDeque<[f32; 2]>,
}
impl InputPatternHistory {
    pub(crate) fn new() -> Self {
        Self {
            occurrence: Arc::new(()),
            // Native constructor reserves 201 slots; no samples are valid yet.
            // Allocation capacity does not turn its uninitialized slots into zeros.
            points: VecDeque::with_capacity(201),
        }
    }
    /// Native allocation201 with an empty sentinel: at most200 valid points.
    pub(crate) fn publish_selected_point(&mut self, point: [f32; 2]) {
        if self.points.len() == 200 {
            self.points.pop_back();
        }
        self.points.push_front(point);
    }
    pub(crate) fn len(&self) -> usize {
        self.points.len()
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InputPatternStick {
    Left,
    Right,
}
impl InputPatternStick {
    pub(crate) const fn collection(self) -> u64 {
        match self {
            Self::Left => 0x0caf_1482_a610_1fa3,
            Self::Right => 0xedfa_d6a7_3a27_9045,
        }
    }
}
pub(crate) trait InputPatternWait {
    type Error;
    fn waiting_limit(&mut self) -> Result<u8, Self::Error>;
}
/// Same immutable generated recognizer instance captured at ctor826def70.
/// Singleton/editor replacement is outside this retained binding boundary.
pub(crate) struct InputPatternVaultAttributes {
    instance: AttributeRuntimeInstance,
}
impl InputPatternVaultAttributes {
    pub(crate) fn bind(vault: &Arc<AttributeRuntimeVault>, stick: InputPatternStick) -> Self {
        Self {
            instance: vault.instance(0x7f08_6875_d621_c88c, stick.collection()),
        }
    }
}
impl InputPatternWait for InputPatternVaultAttributes {
    type Error = AttributeRuntimeError;
    fn waiting_limit(&mut self) -> Result<u8, Self::Error> {
        Ok(self
            .instance
            .try_cached_layout(0x30)?
            .map_or(0, |data| data.bytes()[0x21]))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InputPatternCvar {
    MinError,
    MaxError,
    FastestRatio,
    SlowestRatio,
}
pub(crate) trait InputPatternCvars {
    type Target;
    type Error;
    fn capture(&mut self, cvar: InputPatternCvar) -> Result<Self::Target, Self::Error>;
    fn value(&mut self, target: &Self::Target) -> Result<f32, Self::Error>;
}
#[derive(Debug)]
pub(crate) enum InputPatternMatchError<W, C> {
    ForeignHistory,
    MissingHistory,
    Wait(W),
    Cvar(C),
}
impl<W: fmt::Display, C: fmt::Display> fmt::Display for InputPatternMatchError<W, C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ForeignHistory => f.write_str("matcher received another history occurrence"),
            Self::MissingHistory => f.write_str("held matcher has no current history point"),
            Self::Wait(e) => write!(f, "recognizer waiting limit: {e}"),
            Self::Cvar(e) => write!(f, "recognizer cvar: {e}"),
        }
    }
}
#[derive(Clone, Debug)]
pub(crate) struct InputPatternMatch {
    pub(crate) pattern: Option<Arc<InputPattern>>,
    pub(crate) normalized_speed: f32,
    pub(crate) error: f32,
    pub(crate) time: f32,
}
impl InputPatternMatch {
    /// Explicit caller record initialization; `FindMatch` early exits retain it.
    pub(crate) const fn empty() -> Self {
        Self {
            pattern: None,
            normalized_speed: 0.0,
            error: 0.0,
            time: 0.0,
        }
    }
    fn clear_for_pick(&mut self) {
        self.pattern = None;
        self.normalized_speed = 0.0;
        self.error = 1_000_000.0;
        self.time = 0.0;
    }
}
struct PatternNode {
    pattern: Arc<InputPattern>,
    candidate: bool,
    finished: bool,
    point_index: usize,
    total_time: u16,
    waiting: u8,
    error: f32,
    first: [f32; 2],
    tolerance: f32,
}
impl PatternNode {
    fn new(pattern: &Arc<InputPattern>) -> Self {
        Self {
            pattern: Arc::clone(pattern),
            candidate: false,
            finished: false,
            point_index: pattern.points().len() - 1,
            total_time: 0,
            waiting: 0,
            error: 0.0,
            first: *pattern.points().last().expect("parser validates points"),
            tolerance: pattern.squared_tolerance(),
        }
    }
    fn set_candidate(&mut self, value: bool) {
        self.candidate = value;
        self.total_time = 0;
        self.waiting = 0;
        self.error = 0.0;
    }
    fn increment_time(&mut self) {
        self.total_time = self.total_time.wrapping_add(1) & 0x3ff;
    }
}
pub(crate) struct InputPatternMatcher<A> {
    history: Arc<()>,
    nodes: Vec<PatternNode>,
    attributes: A,
    matched_last_update: bool,
    last_pattern: Option<Arc<InputPattern>>,
}
fn not_greater(left: f32, right: f32) -> bool {
    left.partial_cmp(&right) != Some(Ordering::Greater)
}
fn distance_squared(target: [f32; 2], current: [f32; 2]) -> f32 {
    let x = target[0] - current[0];
    let y = target[1] - current[1];
    let x2 = x * x;
    let y2 = y * y;
    x2 + y2
}
fn fsel(test: f32, nonnegative: f32, negative: f32) -> f32 {
    if test >= 0.0 { nonnegative } else { negative }
}
impl<A: InputPatternWait> InputPatternMatcher<A> {
    pub(crate) fn new(
        patterns: &InputPatterns,
        history: &InputPatternHistory,
        attributes: A,
    ) -> Self {
        Self {
            history: Arc::clone(&history.occurrence),
            nodes: patterns.patterns().iter().map(PatternNode::new).collect(),
            attributes,
            matched_last_update: false,
            last_pattern: None,
        }
    }
    fn remove_all_candidates(&mut self) {
        for node in &mut self.nodes {
            node.set_candidate(false);
            node.finished = false;
        }
    }
    /// `IsHoldingLastMatchedPattern826df9e8`. No name publication on false.
    pub(crate) fn try_holding(
        &mut self,
        history: &InputPatternHistory,
    ) -> Result<Option<Arc<InputPattern>>, InputPatternMatchError<A::Error, std::convert::Infallible>>
    {
        if !Arc::ptr_eq(&self.history, &history.occurrence) {
            return Err(InputPatternMatchError::ForeignHistory);
        }
        let Some(pattern) = &self.last_pattern else {
            return Ok(None);
        };
        let current = history
            .points
            .front()
            .ok_or(InputPatternMatchError::MissingHistory)?;
        // Source subtracts tolerance before ordered comparison with zero.
        let difference =
            distance_squared(*current, pattern.points()[0]) - pattern.squared_tolerance();
        if difference > 0.0 {
            self.last_pattern = None;
            Ok(None)
        } else {
            Ok(Some(Arc::clone(pattern)))
        }
    }
    fn update_patterns(&mut self, current: [f32; 2]) -> Result<(), A::Error> {
        for node in &mut self.nodes {
            if !node.candidate {
                let error = distance_squared(node.first, current);
                if not_greater(error, node.tolerance) {
                    node.set_candidate(true);
                    node.point_index = node.pattern.points().len() - 2;
                    node.error = error;
                    node.increment_time();
                }
            } else if !node.finished {
                let points = node.pattern.points();
                let error = distance_squared(points[node.point_index], current);
                let next_error = distance_squared(points[node.point_index + 1], current);
                if not_greater(error, node.tolerance) {
                    node.error += error;
                    node.increment_time();
                    if node.point_index == 0 {
                        node.finished = true;
                    } else {
                        node.point_index -= 1;
                        node.waiting = 0;
                    }
                } else {
                    if points.len() - node.point_index != 2
                        || !not_greater(next_error, node.tolerance)
                    {
                        node.increment_time();
                        node.waiting = node.waiting.wrapping_add(1) & 0x3f;
                    }
                    if node.waiting > self.attributes.waiting_limit()? {
                        node.set_candidate(false);
                        node.finished = false;
                    }
                }
            }
        }
        Ok(())
    }
    fn pick_winner<C: InputPatternCvars>(&self, cvars: &mut C) -> Result<Option<usize>, C::Error> {
        if self.nodes.is_empty() {
            return Ok(None);
        }
        let min_target = cvars.capture(InputPatternCvar::MinError)?;
        let max_target = cvars.capture(InputPatternCvar::MaxError)?;
        let mut winner = None;
        let mut best = 0.0;
        for (index, node) in self.nodes.iter().enumerate() {
            if !node.finished {
                continue;
            }
            let min = cvars.value(&min_target)?;
            let min_test = min - node.error;
            let max = cvars.value(&max_target)?;
            let n = f32::from(u16::try_from(node.pattern.points().len()).expect("bounded pattern"));
            let a = fsel(min_test, min, node.error);
            let b = a / n;
            let n2 = n * n;
            let max_test = max - b;
            let n3 = n2 * n;
            let c = fsel(max_test, b, max);
            let n4 = n3 * n;
            let denominator = c * f32::from(node.total_time);
            let score = n4 / denominator;
            if winner.is_none() || score > best {
                winner = Some(index);
                best = score;
            }
        }
        Ok(winner)
    }
    /// Source callbacks preserve completed earlier node/output stores on error.
    /// Caller scheduling remains explicit; this does not emit `PlayerInput` events.
    pub(crate) fn try_find_match<C: InputPatternCvars>(
        &mut self,
        history: &InputPatternHistory,
        cvars: &mut C,
        output: &mut InputPatternMatch,
    ) -> Result<bool, InputPatternMatchError<A::Error, C::Error>> {
        use InputPatternMatchError as E;
        if !Arc::ptr_eq(&self.history, &history.occurrence) {
            return Err(E::ForeignHistory);
        }
        if self.matched_last_update {
            self.remove_all_candidates();
            self.matched_last_update = false;
            return Ok(false);
        }
        if history.len() <= 1 {
            return Ok(false);
        }
        self.update_patterns(history.points[0]).map_err(E::Wait)?;
        output.clear_for_pick();
        let Some(index) = self.pick_winner(cvars).map_err(E::Cvar)? else {
            return Ok(false);
        };
        let node = &self.nodes[index];
        output.pattern = Some(Arc::clone(&node.pattern));
        output.time = f32::from(node.total_time);
        output.error = node.error;
        let fastest = cvars
            .capture(InputPatternCvar::FastestRatio)
            .map_err(E::Cvar)?;
        let fastest_value = cvars.value(&fastest).map_err(E::Cvar)?;
        let ratio = output.time
            / f32::from(u16::try_from(node.pattern.points().len()).expect("bounded pattern"));
        if not_greater(ratio, fastest_value) {
            output.normalized_speed = 1.0;
        } else {
            let slowest = cvars
                .capture(InputPatternCvar::SlowestRatio)
                .map_err(E::Cvar)?;
            let slowest_value = cvars.value(&slowest).map_err(E::Cvar)?;
            if ratio.partial_cmp(&slowest_value) == Some(Ordering::Less) {
                let slowest_value = cvars.value(&slowest).map_err(E::Cvar)?;
                let reciprocal = 1.0 / (fastest_value - slowest_value);
                let product = slowest_value * reciprocal;
                output.normalized_speed = reciprocal.mul_add(ratio, -product);
            } else {
                output.normalized_speed = 0.0;
            }
        }
        self.last_pattern = output.pattern.as_ref().map(Arc::clone);
        self.matched_last_update = true;
        Ok(true)
    }
}
#[cfg(test)]
#[path = "input_pattern_matcher_tests.rs"]
mod tests;
