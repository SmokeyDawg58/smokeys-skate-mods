use super::*;
#[derive(Default)]
struct Wait {
    calls: usize,
    fail: bool,
    value: u8,
}
impl InputPatternWait for Wait {
    type Error = &'static str;
    fn waiting_limit(&mut self) -> Result<u8, Self::Error> {
        self.calls += 1;
        if self.fail {
            Err("wait")
        } else {
            Ok(self.value)
        }
    }
}
#[derive(Default)]
struct Cvars {
    calls: Vec<(bool, InputPatternCvar)>,
    fail: Option<usize>,
    fastest: Option<f32>,
    slowest: Option<f32>,
}
impl Cvars {
    fn call(&mut self, capture: bool, cvar: InputPatternCvar) -> Result<(), &'static str> {
        let index = self.calls.len();
        self.calls.push((capture, cvar));
        if self.fail == Some(index) {
            Err("cvar")
        } else {
            Ok(())
        }
    }
}
impl InputPatternCvars for Cvars {
    type Target = InputPatternCvar;
    type Error = &'static str;
    fn capture(&mut self, cvar: InputPatternCvar) -> Result<Self::Target, Self::Error> {
        self.call(true, cvar)?;
        Ok(cvar)
    }
    fn value(&mut self, target: &Self::Target) -> Result<f32, Self::Error> {
        self.call(false, *target)?;
        Ok(match target {
            InputPatternCvar::MinError | InputPatternCvar::MaxError => 0.15,
            InputPatternCvar::FastestRatio => self.fastest.unwrap_or(1.75),
            InputPatternCvar::SlowestRatio => self.slowest.unwrap_or(4.4),
        })
    }
}
fn patterns() -> Arc<InputPatterns> {
    InputPatterns::parse(b"pattern Same tolerance_dist .1 coord 0 1 coord 0 -1 pattern Same tolerance_dist .1 coord 0 1 coord 0 -1").unwrap()
}
#[test]
fn actual_ordered_path_first_tie_cooldown_and_held_release() {
    let patterns = patterns();
    let mut history = InputPatternHistory::new();
    let mut matcher = InputPatternMatcher::new(&patterns, &history, Wait::default());
    let mut cvars = Cvars::default();
    let mut output = InputPatternMatch::empty();
    history.publish_selected_point([0.0, 1.0]);
    assert!(
        !matcher
            .try_find_match(&history, &mut cvars, &mut output)
            .unwrap()
    );
    assert!(cvars.calls.is_empty());
    history.publish_selected_point([0.0, 1.0]);
    assert!(
        !matcher
            .try_find_match(&history, &mut cvars, &mut output)
            .unwrap()
    );
    history.publish_selected_point([0.0, -1.0]);
    assert!(
        matcher
            .try_find_match(&history, &mut cvars, &mut output)
            .unwrap()
    );
    assert!(Arc::ptr_eq(
        output.pattern.as_ref().unwrap(),
        &patterns.patterns()[0]
    ));
    assert_eq!(output.time.to_bits(), 2.0_f32.to_bits());
    assert_eq!(output.normalized_speed.to_bits(), 1.0_f32.to_bits());
    assert!(matcher.try_holding(&history).unwrap().is_some());
    let old_calls = cvars.calls.len();
    assert!(
        !matcher
            .try_find_match(&history, &mut cvars, &mut output)
            .unwrap()
    );
    assert_eq!(cvars.calls.len(), old_calls);
    assert!(output.pattern.is_some());
    history.publish_selected_point([f32::NAN, -1.0]);
    assert!(matcher.try_holding(&history).unwrap().is_some());
    history.publish_selected_point([1.0, 1.0]);
    assert!(matcher.try_holding(&history).unwrap().is_none());
}
#[test]
fn history_occurrence_and_native_capacity_are_retained() {
    let p = patterns();
    let mut history = InputPatternHistory::new();
    let mut matcher = InputPatternMatcher::new(&p, &history, Wait::default());
    let other = InputPatternHistory::new();
    assert!(matches!(
        matcher.try_holding(&other),
        Err(InputPatternMatchError::ForeignHistory)
    ));
    for index in 0_u16..=200 {
        history.publish_selected_point([f32::from(index), 0.0]);
    }
    assert_eq!(history.len(), 200);
    assert_eq!(
        history.points.back().unwrap()[0].to_bits(),
        1.0_f32.to_bits()
    );
}
#[test]
fn wait_failure_retains_increment_and_prior_output_then_masked_wrap() {
    let p = patterns();
    let mut history = InputPatternHistory::new();
    let mut matcher = InputPatternMatcher::new(
        &p,
        &history,
        Wait {
            fail: true,
            ..Wait::default()
        },
    );
    history.publish_selected_point([0.0, 1.0]);
    history.publish_selected_point([0.0, 1.0]);
    let mut c = Cvars::default();
    let mut output = InputPatternMatch::empty();
    matcher
        .try_find_match(&history, &mut c, &mut output)
        .unwrap();
    output.error = 77.0;
    history.publish_selected_point([1.0, 0.0]);
    assert!(matches!(
        matcher.try_find_match(&history, &mut c, &mut output),
        Err(InputPatternMatchError::Wait("wait"))
    ));
    assert_eq!(matcher.nodes[0].waiting, 1);
    assert_eq!(matcher.nodes[0].total_time, 2);
    assert_eq!(matcher.nodes[1].total_time, 1);
    assert_eq!(output.error.to_bits(), 77.0_f32.to_bits());
    matcher.attributes.fail = false;
    matcher.attributes.value = 63;
    matcher.nodes[0].waiting = 63;
    matcher.nodes[0].total_time = 1023;
    matcher
        .try_find_match(&history, &mut c, &mut output)
        .unwrap();
    assert_eq!(matcher.nodes[0].waiting, 0);
    assert_eq!(matcher.nodes[0].total_time, 0);
}
#[test]
fn first_segment_wait_suppression_and_unordered_target_admission() {
    let p = patterns();
    let mut h = InputPatternHistory::new();
    let mut m = InputPatternMatcher::new(
        &p,
        &h,
        Wait {
            value: 63,
            ..Wait::default()
        },
    );
    h.publish_selected_point([0.0, 1.0]);
    h.publish_selected_point([0.0, 1.0]);
    let mut c = Cvars::default();
    let mut out = InputPatternMatch::empty();
    m.try_find_match(&h, &mut c, &mut out).unwrap();
    m.try_find_match(&h, &mut c, &mut out).unwrap();
    assert_eq!(m.nodes[0].total_time, 1);
    assert_eq!(m.attributes.calls, 2);
    h.publish_selected_point([f32::NAN, 0.0]);
    assert!(m.try_find_match(&h, &mut c, &mut out).unwrap());
    assert!(out.error.is_nan());
    assert!(Arc::ptr_eq(out.pattern.as_ref().unwrap(), &p.patterns()[0]));
}
#[test]
fn cvar_failure_keeps_winner_publication_without_success_or_cooldown() {
    let p = patterns();
    let mut h = InputPatternHistory::new();
    let mut m = InputPatternMatcher::new(&p, &h, Wait::default());
    h.publish_selected_point([0.0, 1.0]);
    h.publish_selected_point([0.0, 1.0]);
    let mut c = Cvars::default();
    let mut out = InputPatternMatch::empty();
    m.try_find_match(&h, &mut c, &mut out).unwrap();
    h.publish_selected_point([0.0, -1.0]);
    c.calls.clear();
    c.fail = Some(6);
    assert!(matches!(
        m.try_find_match(&h, &mut c, &mut out),
        Err(InputPatternMatchError::Cvar("cvar"))
    ));
    assert!(out.pattern.is_some());
    assert_eq!(out.time.to_bits(), 2.0_f32.to_bits());
    assert!(!m.matched_last_update);
    assert!(m.last_pattern.is_none());
    assert_eq!(c.calls[6], (true, InputPatternCvar::FastestRatio));
}
#[test]
fn late_slowest_reread_and_nan_fastest_branch() {
    let p = patterns();
    let mut h = InputPatternHistory::new();
    let mut m = InputPatternMatcher::new(&p, &h, Wait::default());
    h.publish_selected_point([0.0, 1.0]);
    h.publish_selected_point([0.0, 1.0]);
    let mut c = Cvars {
        fastest: Some(0.5),
        ..Cvars::default()
    };
    let mut out = InputPatternMatch::empty();
    m.try_find_match(&h, &mut c, &mut out).unwrap();
    h.publish_selected_point([0.0, -1.0]);
    c.calls.clear();
    assert!(m.try_find_match(&h, &mut c, &mut out).unwrap());
    let tail = &c.calls[c.calls.len() - 3..];
    assert_eq!(
        tail,
        &[
            (true, InputPatternCvar::SlowestRatio),
            (false, InputPatternCvar::SlowestRatio),
            (false, InputPatternCvar::SlowestRatio)
        ]
    );
    let reciprocal = 1.0_f32 / (0.5 - 4.4);
    let product = 4.4 * reciprocal;
    assert_eq!(
        out.normalized_speed.to_bits(),
        reciprocal.mul_add(1.0, -product).to_bits()
    );
    m.matched_last_update = false;
    c.fastest = Some(f32::NAN);
    c.calls.clear();
    assert!(m.try_find_match(&h, &mut c, &mut out).unwrap());
    assert_eq!(out.normalized_speed.to_bits(), 1.0_f32.to_bits());
    assert!(
        !c.calls
            .iter()
            .any(|(_, kind)| *kind == InputPatternCvar::SlowestRatio)
    );
}
#[test]
fn actual_cached_wait_layout_binding_and_default_area() {
    use crate::attribute_runtime::tests::{collection, graph, set_dependency_bytes};
    let class = 0x7f08_6875_d621_c88c;
    assert_eq!(class, skate2_formats::attrib::string_hash64("recognizer"));
    let mut vault = graph(class, vec![]);
    collection(
        &mut vault,
        class,
        InputPatternStick::Left.collection(),
        0,
        Some(0),
        vec![],
    );
    collection(
        &mut vault,
        class,
        InputPatternStick::Right.collection(),
        0,
        Some(0x40),
        vec![],
    );
    let mut bytes = vec![0; 0x80];
    bytes[0x21] = 7;
    bytes[0x61] = 19;
    set_dependency_bytes(&mut vault, 1, bytes);
    let vault = Arc::new(vault);
    assert_eq!(
        InputPatternVaultAttributes::bind(&vault, InputPatternStick::Left)
            .waiting_limit()
            .unwrap(),
        7
    );
    assert_eq!(
        InputPatternVaultAttributes::bind(&vault, InputPatternStick::Right)
            .waiting_limit()
            .unwrap(),
        19
    );
    let empty = Arc::new(graph(class, vec![]));
    assert_eq!(
        InputPatternVaultAttributes::bind(&empty, InputPatternStick::Left)
            .waiting_limit()
            .unwrap(),
        0
    );
}
