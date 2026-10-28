# Captured-assembly wheel-position copies

Unregistered source/output drafts continue82b301a4..30284. Native f430 captures
Body+18 assembly descriptor and f438 retains it at stack var270. Reload3019c
feeds the four selected Parts+8 reads. The descriptor identifies one actual
retained assembly, but it does not snapshot any current Part body or position.

For wheel0, Parts pointer301a4 and body pointer301a8 precede latch301ac;
body+10 position loads301b0, followed by the assertion and Reckoning+d0
store301d8. For wheel1, Parts301dc precedes latch301e0, but body301e8 and
position301ec follow it, before the assertion and Reckoning+e0 store30210.
Wheels2/3 repeat that latter order at30214..248 and3024c..284, writing
Reckoning+f0/+100. Thus the draft distinguishes the first pointer ordering
rather than flattening the four cases into eager body snapshots.

The planned descriptor token clones the actual SimulationSkaterAssemblyHandle
at initial Fill capture and propagates it through all intermediate tokens.
The final caller verifies same_as before touching output, validates the same
retained session for each Parts capture, resolves each current body at its
selected source read, and reads actual pool position XYZ through the existing
direct body getter. Intrusive body W remains unknown. The historically named
contact-spy getter does not itself require or create a ContactSpyFrame.

Focused draft tests cover a changed current Part body under the same captured
descriptor, partial stores when wheel2 fails, unknown W/signed-zero retention,
and first-wheel position read before locked-output refusal. The changed-body
test exercises the typed source contract; real assembly-handle mismatch coverage
is requested in the parent-owned mounted corpus using its existing handles.
No fake public handle constructor is added just to build that test.

All files remain unregistered pending the parent gate. Initial capture and
shared continuation propagation have not yet been edited. The output token
stops before outer GetSkateboardBody/time-since-wheels at30288.
