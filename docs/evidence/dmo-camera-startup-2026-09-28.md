# DMO initial census camera ownership

Runtime failure: `LivingWorldDmo / Camera(CameraProjection(NoCompletedSelectedUpdate))`
immediately on launch. This is an integration mistake: camera_lanes requested
the renderer's completed CameraMan projection before the first selected tick.
Both initial census and normal Update1/PostTransforms need the simulation
camera, not a newly completed render projection.

## Live IDA verification

- Manager.GetCameraPosition `0x82703340` and GetCameraHeading `0x82703410`
  read CameraManager.mCameraList[0], vtable offsets 0x34 and 0x3c.
- CameraManager constructor `0x825849c8`, allocation/call `0x82584b0c..b18`,
  creates BasicCameras before GameCameraManager. This is not a CameraMan frame.
- BasicCamera vtable `0x821f8738`: GetPosition `0x829d3190` copies this+0x50;
  heading `0x82e6a6a8` copies this+0x40. The latter has a misleading folded
  Navigation.GetSeekDestination symbol; raw instructions establish the getter.
- BasicCamera constructor `0x829d2fa0` sets position (-7,1,0,0), up (0,1,0,0),
  and target (0,0,0,0). SetTarget `0x829d2ab8` normalizes target-position using
  dot3 and two reciprocal-square-root refinements, then sets X=cross(up,Z).
  No additional X or Y normalization. Decoded VMX operands checked at
  `0x829d2af0..b98`; use existing ReckoningNumeric normalization/cross helpers.
- CameraManager.Update `0x82584448` calls GameCameraManager.Update at
  `0x82584588`; GetFrame at `0x8258464c` then first BasicCamera.SetMatrix at
  `0x82584664`. SetMatrix `0x829d3158` copies all four matrix columns.

## Change and limits

The existing CameraStartupFields owner retains a simulation_camera_matrix.
Selected capture initializes the source BasicCamera pose before constructing
GameCameraManager components. Successful selected CameraMan update publishes
its matrix into that same slot. Local-skater notifications leave it intact.
SkaterManager's existing observer exposes this matrix to DMO camera_lanes;
initial census and subsequent ticks share the accessor. No second camera rig,
extra update, synthetic spawn-derived pose, or bypass of render readiness.

CameraProducerProjection's NoCompletedSelectedUpdate guard remains unchanged.
The new slot represents the previously missing BasicCamera simulation snapshot;
it does not implement native replay/presentation interpolation. Existing host
numeric limitations (including Xenon reciprocal-sqrt estimate bit parity) remain.

Validation: scoped rustfmt syntax parsing passed all four edited Rust files.
No Cargo or gameplay run.
Rebuild and initial-census/normal-tick runtime verification remain pending.
