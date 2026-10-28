//! Native diagnostic window. Display cadence is not the simulation timestep.
use crate::gpu_renderer::{GpuObject, GpuRenderer, RenderResult};
use std::{sync::Arc, time::Instant};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

/// Host owns input mapping, fixed-step scheduling and actual retained scene/pose.
pub trait RenderWindowClient {
    fn initialize(&mut self, renderer: &GpuRenderer) -> RenderResult<Vec<GpuObject>>;
    fn window_event(&mut self, event: &WindowEvent) -> RenderResult<()>;
    fn frame(
        &mut self,
        renderer: &GpuRenderer,
        objects: &mut Vec<GpuObject>,
        now: Instant,
    ) -> RenderResult<()>;

    /// A client-defined completed result ready for presentation. Inspection
    /// and other window clients retain the ordinary presented-frame bound.
    fn completion_reached(&self) -> bool {
        false
    }

    /// Whether capture and exit are bounded by client completions instead of
    /// the window's ordinary presented-frame count.
    fn completion_bound_active(&self) -> bool {
        false
    }

    /// Called only after a frame containing the requested client completion
    /// was successfully presented (and captured, when capture was requested).
    fn record_presented_completion(&mut self) {}
}

#[derive(Default)]
pub struct RenderWindowOptions {
    /// Number of successfully presented frames; None runs until close.
    pub frame_limit: Option<u64>,
    /// Capture the final finite frame, or first frame for an interactive run.
    pub capture: Option<std::path::PathBuf>,
    /// Explicit diagnostic title for this window client.
    pub title: Option<&'static str>,
}

pub fn run_with_options(
    mut client: impl RenderWindowClient + 'static,
    mut options: RenderWindowOptions,
) -> RenderResult<()> {
    if options.frame_limit == Some(0) {
        return Err("frame limit must be positive".into());
    }
    let event_loop = EventLoop::new()?;
    let window = Arc::new(
        WindowBuilder::new()
            .with_title(
                options
                    .title
                    .unwrap_or("Skate 2 — original geometry / diagnostic camera and shading"),
            )
            .with_inner_size(winit::dpi::LogicalSize::new(1280, 720))
            .build(&event_loop)?,
    );
    let mut renderer = pollster::block_on(GpuRenderer::new(Arc::clone(&window)))?;
    let mut objects = client.initialize(&renderer)?;
    let failure = Arc::new(std::sync::Mutex::new(None::<String>));
    let event_failure = Arc::clone(&failure);
    let mut presented = 0_u64;
    event_loop.run(move |event, target| {
        target.set_control_flow(ControlFlow::Wait);
        if let Event::WindowEvent { window_id, event } = event {
            if window_id != window.id() {
                return;
            }
            let result = client.window_event(&event).and_then(|()| {
                match event {
                    WindowEvent::CloseRequested => target.exit(),
                    WindowEvent::KeyboardInput { event, .. }
                        if event.state == winit::event::ElementState::Pressed
                            && event.physical_key
                                == winit::keyboard::PhysicalKey::Code(
                                    winit::keyboard::KeyCode::Escape,
                                ) =>
                    {
                        target.exit();
                    }
                    WindowEvent::Resized(size) => {
                        renderer.resize(size.width, size.height);
                        window.request_redraw();
                    }
                    WindowEvent::RedrawRequested => {
                        client.frame(&renderer, &mut objects, Instant::now())?;
                        window.pre_present_notify();
                        if window.inner_size().width == 0 || window.inner_size().height == 0 {
                            return Ok(());
                        }
                        let completion_reached = client.completion_reached();
                        let capture_now = options.capture.is_some()
                            && if client.completion_bound_active() {
                                completion_reached
                            } else {
                                options
                                    .frame_limit
                                    .is_none_or(|limit| presented + 1 == limit)
                            };
                        let did_present = if capture_now {
                            renderer.render_capture(
                                &objects,
                                options.capture.as_deref().ok_or("missing capture path")?,
                            )?;
                            options.capture = None;
                            true
                        } else {
                            match renderer.render(&objects) {
                                Ok(()) => true,
                                Err(wgpu::SurfaceError::Timeout) => false,
                                Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                                    let size = window.inner_size();
                                    renderer.resize(size.width, size.height);
                                    false
                                }
                                Err(error) => return Err(error.into()),
                            }
                        };
                        if did_present {
                            presented += 1;
                            if completion_reached {
                                client.record_presented_completion();
                            }
                        }
                        if options.frame_limit == Some(presented)
                            || (completion_reached && did_present)
                        {
                            target.exit();
                            return Ok(());
                        }
                        if window.inner_size().width != 0 && window.inner_size().height != 0 {
                            window.request_redraw();
                        }
                    }
                    _ => {}
                }
                Ok(())
            });
            if let Err(error) = result {
                if let Ok(mut failure) = event_failure.lock() {
                    *failure = Some(error.to_string());
                }
                target.exit();
            }
        } else if matches!(event, Event::Resumed) {
            window.request_redraw();
        }
    })?;
    let failure = failure
        .lock()
        .map_err(|_| "window failure holder poisoned")?
        .take();
    if let Some(message) = failure {
        return Err(message.into());
    }
    Ok(())
}
