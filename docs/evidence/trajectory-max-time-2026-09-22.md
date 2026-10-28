# Maximum trajectory duration

`physical_player_trajectory_max_time.rs` implements
TrajectorySelector::CalcMaxTimeForTrajectory82dc0118 on the retained global
trajectory attribute capability and linked cvar references. Independent reviewer
approved raw arithmetic, read order and four focused tests before registration.

Source reads layout+1ac cap before FETCH63673fb6da092b7e; missing FETCH uses
DefaultDataArea4, while null generated layout follows constructor828b1068's
DefaultDataArea24c. Actual malformed scalar data fails instead of becoming zero.
Raw01a0/01b4 negate the parameter twice;01b8 rounds its product with c19ccccd,
then01bc FMSUBS computes velocityY squared minus that product. Strict-positive
01c8 excludes zero, negatives and unordered values.

Rsqrt estimate022c occurs before target/value reads0254/025c. Existing
length_from_squared_and_estimate preserves two refinement rounds and final
multiply/zero selection. Duration=(-velocityY-length)*bdd0fac6. FSEL02b4 chooses
cap only when duration-cap>=0, including signed zero; unordered keeps duration.
Only enabled debug drawing requires the actual overlay.

The live cvar is Physics_Trajectory/kbDebugDraw, title Debug Draw, initialfalse,
null callback: reference830da824, initializer82f967f0, table82fd5c6c. New index240
is inserted in source registration order before219(table5d44), followed by
215(5d4c),217(5d50),218(5d54),216(5d58),86(5d90). Private Launch uses this SAME
reference at82dc2c6c; no separate cached tuning or debug boolean is created.

Focused tests cover cap including NaN/signed-zero, invalid discriminant refusal,
estimate failure before cvar access, and actual cvar link/reload plus conditional
overlay requirement. Compilation/tests are pending the current integration gate.
Native console rsqrt seed/denormal/NaN payload limitations remain those documented
by the existing numeric service. Full private Launch is still separate work.
