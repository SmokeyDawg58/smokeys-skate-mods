//! Source named chase conditions with branch-lazy reads of actual owners.
use super::{ChaseeState,TargetableId};
#[derive(Clone,Copy)]
pub(crate) enum TargetQuery { State, Position, WipedOut, Protector, Knowledge }
pub(crate) struct ChaseTargetView<'a> {
    pub(crate) state:&'a ChaseeState,
    pub(crate) group_end_reason:i32,
    pub(crate) self_entity_exists:bool,
    pub(crate) position:Option<[f32;3]>,
    pub(crate) wiped_out:Option<bool>,
    pub(crate) has_protector:Option<bool>,
    pub(crate) knowledge_flags:Option<u8>,
}
/// A selected condition requests only facts its native callback reads.
pub(crate) trait ChaseConditionServices {
    fn new_chases_allowed(&self)->bool;
    fn self_id(&self)->TargetableId;
    fn end_reason(&self)->i32;
    fn with_target<R>(&mut self,query:TargetQuery,read:impl FnOnce(Option<ChaseTargetView<'_>>)->R)->Result<R,String>;
    fn can_want_target_add_chaser(&mut self)->Result<bool,String>;
    fn position(&self)->Result<[f32;3],String>;
    fn escape_distance(&self)->Result<f32,String>;
    fn cannot_reach(&self)->bool;
    fn should_block(&self)->Result<bool,String>;
    fn give_up_after_takedowns(&self)->Result<bool,String>;
    fn takedown_count(&self)->Result<u32,String>;
    fn takedowns_to_give_up(&self)->Result<u32,String>;
    fn patrol_interest_timer(&self)->f32;
    fn in_own_zone(&self,include_buffer:bool)->Result<bool,String>;
}
pub(crate) fn evaluate(name:&str,source:&mut impl ChaseConditionServices)->Result<Option<bool>,String>{
    use TargetQuery as Q;
    Ok(Some(match name {
        "CanNewChaseStart"=>source.new_chases_allowed(), //82e6edc0
        "CanChaseeAddNewChaser"=>source.can_want_target_add_chaser()?, //825ef830
        "IsChaserEndingChase"=>{
            let reason=source.end_reason();
            source.with_target(Q::State,|target|target.is_some_and(|t|t.group_end_reason!=-1||reason!=-1))?
        }
        "ChaseeIsWipedOut"=>source.with_target(Q::WipedOut,|t|t.and_then(|t|t.wiped_out).unwrap_or(false))?,
        "ChaseeHasProtector"=>source.with_target(Q::Protector,|t|t.and_then(|t|t.has_protector).unwrap_or(false))?,
        "IsPrimaryChaser"=>{
            let id=source.self_id();
            source.with_target(Q::State,|t|t.is_some_and(|t|t.state.chasers.first().is_some_and(|c|c.id==id)))?
        }
        "ChasersAreScared"=>source.with_target(Q::State,|t|t.is_some_and(|t|!t.self_entity_exists&&t.state.scared_timer>=0.))?,
        "CanSeeChasee"=>source.with_target(Q::Knowledge,|t|t.is_some_and(|t|t.knowledge_flags.is_some_and(|f|f&0xc0!=0)))?,
        "CannotReachChaseTarget"=>source.cannot_reach(),
        "ChaserShouldBlock"=>source.should_block()?,
        "ChaserShouldGiveUpDueToTakedowns"=>source.give_up_after_takedowns()?
            &&source.takedown_count()?>=source.takedowns_to_give_up()?,
        "ChaseeEscaped"=>{
            let target=source.with_target(Q::Position,|t|t.and_then(|t|t.position))?;
            if let Some(target)=target {let position=source.position()?;let distance=source.escape_distance()?;
                distance_squared(target,position)-distance*distance>0.
            }else{false}
        }
        "ChaseeCloseEnoughToSatisfyAggressiveChase"=>{
            let target=source.with_target(Q::Position,|t|t.and_then(|t|t.position))?;
            if let Some(target)=target {4.>distance_squared(target,source.position()?)}else{false}
        }
        "PatrolZoneMaintainInterestTimerActive"=>source.patrol_interest_timer()>0.,
        "PatrolZoneOwnerIsInOwnZone"=>source.in_own_zone(false)?,
        "PatrolZoneOwnerIsInOwnZoneBoundaryOrBuffer"=>source.in_own_zone(true)?,
        _=>return Ok(None),
    }))
}
fn distance_squared(a:[f32;3],b:[f32;3])->f32{
    let d=[a[0]-b[0],a[1]-b[1],a[2]-b[2]];
    d[0]*d[0]+d[1]*d[1]+d[2]*d[2]
}
