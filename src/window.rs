use calloop::{channel, EventLoop};
use calloop_wayland_source::WaylandSource;
use cosmic_text::{FontSystem, SwashCache};
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_layer, delegate_output, delegate_registry, delegate_shm,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    shell::{
        WaylandSurface,
        wlr_layer::{
            Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface,
            LayerSurfaceConfigure,
        },
    },
    shm::{slot::SlotPool, Shm, ShmHandler},
};
use tiny_skia::Pixmap;
use wayland_client::{
    globals::registry_queue_init,
    protocol::{wl_output, wl_region, wl_shm, wl_surface},
    Connection, QueueHandle,
};

use crate::{
    cli::Args,
    feed::Candle,
    render::{self, WIN_H},
    state::AppState,
};

struct App {
    // Kept alive for SCTK dispatch; never read directly.
    #[allow(dead_code)]
    compositor: CompositorState,
    #[allow(dead_code)]
    layer_shell: LayerShell,
    output_state: OutputState,
    registry_state: RegistryState,
    shm: Shm,

    layer_surface: LayerSurface,
    pool: SlotPool,
    pixmap: Pixmap,

    font_system: FontSystem,
    swash_cache: SwashCache,
    state: AppState,

    win_w: u32,
    scale: i32,
    configured: bool,
    exit: bool,
}

impl App {
    fn phys(&self) -> (u32, u32) {
        (self.win_w * self.scale as u32, WIN_H * self.scale as u32)
    }

    fn paint_and_attach(&mut self) {
        let (pw, ph) = self.phys();
        if self.pixmap.width() != pw || self.pixmap.height() != ph {
            self.pixmap = Pixmap::new(pw, ph).unwrap();
        }

        render::paint(&mut self.pixmap, &mut self.font_system, &mut self.swash_cache, &self.state, self.scale);

        let stride = pw as i32 * 4;
        let Ok((buffer, canvas)) =
            self.pool.create_buffer(pw as i32, ph as i32, stride, wl_shm::Format::Argb8888)
        else {
            return;
        };

        // tiny-skia premultiplied RGBA → Wayland ARGB8888 LE [B, G, R, A], un-premultiply for transparent pixels.
        for (dst, src) in canvas.chunks_exact_mut(4).zip(self.pixmap.data().chunks_exact(4)) {
            let a = src[3];
            if a == 0 {
                dst.copy_from_slice(&[0, 0, 0, 0]);
            } else if a == 255 {
                dst[0] = src[2]; dst[1] = src[1]; dst[2] = src[0]; dst[3] = 255;
            } else {
                let u = |c: u8| ((c as u32 * 255 + a as u32 / 2) / a as u32).min(255) as u8;
                dst[0] = u(src[2]); dst[1] = u(src[1]); dst[2] = u(src[0]); dst[3] = a;
            }
        }

        let surface = self.layer_surface.wl_surface();
        surface.set_buffer_scale(self.scale);
        surface.damage_buffer(0, 0, pw as i32, ph as i32);
        buffer.attach_to(surface).expect("buffer attach");
        self.state.dirty = false;
    }
}

impl CompositorHandler for App {
    fn scale_factor_changed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, factor: i32) {
        if self.scale != factor { self.scale = factor; self.state.dirty = true; }
    }
    fn transform_changed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: wl_output::Transform) {}
    fn frame(&mut self, _: &Connection, qh: &QueueHandle<Self>, surface: &wl_surface::WlSurface, _: u32) {
        surface.frame(qh, surface.clone());
        if self.state.dirty { self.paint_and_attach(); }
        surface.commit();
    }
    fn surface_enter(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: &wl_output::WlOutput) {}
    fn surface_leave(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: &wl_output::WlOutput) {}
}

impl OutputHandler for App {
    fn output_state(&mut self) -> &mut OutputState { &mut self.output_state }
    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
}

impl LayerShellHandler for App {
    fn closed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &LayerSurface) { self.exit = true; }
    fn configure(&mut self, _: &Connection, qh: &QueueHandle<Self>, _: &LayerSurface, _: LayerSurfaceConfigure, _: u32) {
        self.configured = true;
        self.state.dirty = true;
        { let surface = self.layer_surface.wl_surface(); surface.frame(qh, surface.clone()); }
        self.paint_and_attach();
        self.layer_surface.wl_surface().commit();
    }
}

impl ShmHandler for App {
    fn shm_state(&mut self) -> &mut Shm { &mut self.shm }
}

impl ProvidesRegistryState for App {
    fn registry(&mut self) -> &mut RegistryState { &mut self.registry_state }
    registry_handlers![OutputState];
}

wayland_client::delegate_noop!(App: ignore wl_region::WlRegion);
delegate_compositor!(App);
delegate_output!(App);
delegate_layer!(App);
delegate_shm!(App);
delegate_registry!(App);

pub fn run(args: Args, rx: channel::Channel<Candle>) {
    let font_system = FontSystem::new();
    let swash_cache = SwashCache::new();

    let conn = Connection::connect_to_env().expect("no Wayland display");
    let (globals, event_queue) = registry_queue_init::<App>(&conn).unwrap();
    let qh = event_queue.handle();

    let compositor = CompositorState::bind(&globals, &qh).expect("compositor");
    let layer_shell = LayerShell::bind(&globals, &qh).expect("wlr-layer-shell");
    let shm = Shm::bind(&globals, &qh).expect("wl_shm");

    let wl_surface = compositor.create_surface(&qh);
    let layer_surface = layer_shell.create_layer_surface(&qh, wl_surface, Layer::Top, Some("hlm"), None);

    layer_surface.set_size(args.width, WIN_H);
    layer_surface.set_anchor(Anchor::TOP | Anchor::RIGHT);
    layer_surface.set_margin(8, 8, 0, 0);
    layer_surface.set_keyboard_interactivity(KeyboardInteractivity::None);

    let input_region = compositor.wl_compositor().create_region(&qh, ());
    layer_surface.wl_surface().set_input_region(Some(&input_region));
    layer_surface.wl_surface().commit();

    let win_w = args.width;
    let pool = SlotPool::new(2 * 1024 * 1024, &shm).expect("shm pool");
    let mut app = App {
        compositor,
        layer_shell,
        output_state: OutputState::new(&globals, &qh),
        registry_state: RegistryState::new(&globals),
        shm,
        layer_surface,
        pool,
        pixmap: Pixmap::new(win_w, WIN_H).unwrap(),
        font_system,
        swash_cache,
        state: AppState::new(args.coin, args.interval.to_hl().to_string()),
        win_w,
        scale: 1,
        configured: false,
        exit: false,
    };

    let mut event_loop: EventLoop<App> = EventLoop::try_new().unwrap();
    let handle = event_loop.handle();
    WaylandSource::new(conn, event_queue).insert(handle.clone()).unwrap();

    handle.insert_source(rx, |event, _, app| {
        if let channel::Event::Msg(candle) = event { app.state.push(candle); }
    }).unwrap();

    loop {
        event_loop.dispatch(None, &mut app).unwrap();
        if app.exit { break; }
    }
}
