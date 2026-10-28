//! Source SequenceNode.Eval82d75e70 / SelectorNode.Eval82d76308.
use super::eval::{AnimationEvalCommands, AnimationEvalError as E, AnimationEvalParams};
use super::sequence::FRAME;
use super::{AnimationNodeHandle as H, AnimationTreePool};

impl AnimationTreePool {
    /// Selected result AND next result; the Blend command is unconditional once
    /// the overlap arm is entered, unlike TransitionBlend's boolean gates.
    pub(crate) fn eval_sequence<C: AnimationEvalCommands>(
        &mut self,
        h: &H,
        p: &AnimationEvalParams,
        do_eval: bool,
        commands: &mut C,
    ) -> Result<bool, E<C::Error>> {
        let selected = self.selected_child(h)?;
        if !do_eval {
            return Ok(false);
        }
        let first = self.eval(&selected, p, do_eval, commands)?;
        let speed = self.sequence(h)?.speed;
        let frame = FRAME / speed;
        let length = self.length(&selected)? - frame;
        let time = self.time(&selected)? - frame;
        //75f4c BO12/BI25: GT must hold; NaN asserts after first Eval/time reads.
        if speed.partial_cmp(&f32::from_bits(0x38d1_b717)) != Some(std::cmp::Ordering::Greater) {
            return Err(E::MissingOwner("Sequence positive speed assertion"));
        }
        let mut overlap = if speed < 3.0 {
            0.2 / speed
        } else {
            f32::from_bits(0x3d88_850a)
        };
        //75f94 BO12/BI25 skips assignment only for GT, including NaN replacement.
        if length.partial_cmp(&overlap) != Some(std::cmp::Ordering::Greater) {
            overlap = length;
        }
        let elapsed = (time - length) + overlap;
        let mut second = true;
        if elapsed > 0.0 {
            let n = self.sequence(h)?;
            if let Some(next) = n.next {
                let child = n
                    .children
                    .get(next)
                    .ok_or(E::MissingOwner("Sequence next"))?
                    .animation
                    .clone();
                let ratio = elapsed / overlap;
                let lower = if -ratio >= 0.0 { 0.0 } else { ratio };
                second = self.eval(&child, p, do_eval, commands)?;
                let weight = if 1.0 - lower >= 0.0 { lower } else { 1.0 };
                commands.blend(weight).map_err(E::Command)?;
            }
        }
        Ok(second && first)
    }
    pub(crate) fn eval_selector<C: AnimationEvalCommands>(
        &mut self,
        h: &H,
        p: &AnimationEvalParams,
        do_eval: bool,
        commands: &mut C,
    ) -> Result<bool, E<C::Error>> {
        let selected = self.selected_child(h)?;
        let n = self.selector(h)?;
        //635c BO12/BI26 requires actual next==end, before doEval.
        if n.next.is_some() {
            return Err(E::MissingOwner("Selector next must equal end"));
        }
        if !do_eval {
            return Ok(false);
        }
        if n.transitioning {
            return Err(E::MissingOwner("Selector transition next child"));
        }
        self.eval(&selected, p, do_eval, commands)
    }
}
