# Original-map inspection

This native window shows original Rob's Park geometry and resident diffuse
textures. It is a renderer inspection, with diagnostic unlit shading and a
free inspection camera. It does not yet spawn a skater or run skating physics.

From the repository in PowerShell:

```powershell
$env:SKATE2_GAME_ROOT='<game folder>'
cargo run -p skate2-pc -- --render-inspection
```

Arrow keys move the camera target across the map. PageUp/PageDown change the
view distance. Escape closes the window. The original assets stay external.

For a finite run with a capture:

```powershell
cargo run -p skate2-pc -- --render-inspection --render-frames 3 --render-capture artifacts/map-inspection.bmp
```

September12 validation on Vulkan: 2,085 original mesh contexts,
1,189 draws using original diffuse textures, 896 explicitly solid because
their diffuse textures are not resident, and 88 deferred effects. The actual
counts and adapter are printed. Original lighting, shadows, material alpha,
gameplay camera, character presentation and the complete simulation frame
remain unfinished. Other platforms/backends have not been validated.
