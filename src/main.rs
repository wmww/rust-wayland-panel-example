use gtk4::prelude::*;
use gtk4::{glib, Button};
use std::os::fd::AsRawFd;
use wayland_client::{protocol::wl_registry, Proxy};
use wayland_protocols_wlr::foreign_toplevel::v1::client::{
    zwlr_foreign_toplevel_handle_v1, zwlr_foreign_toplevel_manager_v1,
};

struct AppData {
    button: Button,
}

impl wayland_client::Dispatch<wl_registry::WlRegistry, wayland_client::globals::GlobalListContents>
    for AppData
{
    fn event(
        _: &mut Self,
        _: &wl_registry::WlRegistry,
        _: wl_registry::Event,
        _: &wayland_client::globals::GlobalListContents,
        _: &wayland_client::Connection,
        _: &wayland_client::QueueHandle<AppData>,
    ) {
    }
}

impl wayland_client::Dispatch<zwlr_foreign_toplevel_manager_v1::ZwlrForeignToplevelManagerV1, ()>
    for AppData
{
    fn event(
        _: &mut Self,
        _: &zwlr_foreign_toplevel_manager_v1::ZwlrForeignToplevelManagerV1,
        _: zwlr_foreign_toplevel_manager_v1::Event,
        _: &(),
        _: &wayland_client::Connection,
        _: &wayland_client::QueueHandle<AppData>,
    ) {
    }

    wayland_client::event_created_child!(Self, zwlr_foreign_toplevel_handle_v1::ZwlrForeignToplevelHandleV1, [
        zwlr_foreign_toplevel_manager_v1::EVT_TOPLEVEL_OPCODE => (zwlr_foreign_toplevel_handle_v1::ZwlrForeignToplevelHandleV1, ())
    ]);
}

impl wayland_client::Dispatch<zwlr_foreign_toplevel_handle_v1::ZwlrForeignToplevelHandleV1, ()>
    for AppData
{
    fn event(
        data: &mut Self,
        _registry: &zwlr_foreign_toplevel_handle_v1::ZwlrForeignToplevelHandleV1,
        event: zwlr_foreign_toplevel_handle_v1::Event,
        _: &(),
        _: &wayland_client::Connection,
        _qh: &wayland_client::QueueHandle<AppData>,
    ) {
        match event {
            zwlr_foreign_toplevel_handle_v1::Event::Title { title } => {
                println!("toplevel: {}", title);
                data.button.set_label(&title);
            }
            _ => (),
        }
    }
}

fn main() {
    gtk4::init().unwrap();

    let button = Button::builder()
        .label("Press me!")
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();

    button.connect_clicked(|button| {
        button.set_label("Hello World!");
    });

    let window = gtk4::Window::new();
    window.set_title("Phie Shell".into());
    window.set_child(Some(&button));
    gtk4_layer_shell::init_for_window(&window);
    gtk4_layer_shell::set_anchor(&window, gtk4_layer_shell::Edge::Bottom, true);
    window.show();

    let gdk_wayland_display = gtk4::gdk::Display::default()
        .unwrap()
        .downcast::<gdk4_wayland::WaylandDisplay>()
        .unwrap();
    let wl_display = gdk_wayland_display.wl_display().unwrap();
    let wl_display_ptr = wl_display.id().as_ptr();
    let connection = wayland_client::Connection::from_backend(unsafe {
        wayland_client::backend::Backend::from_foreign_display(wl_display_ptr as _)
    });
    /*let mut event_queue = conn.new_event_queue();
    let qh = event_queue.handle();
    println!("getting registry");
    let _registry = wl_display.get_registry(&qh, ());

    let mut app_data = AppData::new();
    event_queue.roundtrip(&mut app_data).unwrap();
    event_queue.roundtrip(&mut app_data).unwrap();
    println!("roundtrip done");
    */
    let (globals, mut event_queue) =
        wayland_client::globals::registry_queue_init::<AppData>(&connection).unwrap();
    let qh = event_queue.handle();
    let _manager = globals
        .bind::<zwlr_foreign_toplevel_manager_v1::ZwlrForeignToplevelManagerV1, _, _>(
            &qh,
            core::ops::RangeInclusive::new(1, 1),
            (),
        )
        .unwrap();
    let mut app_data = AppData { button };
    //event_queue.roundtrip(&mut app_data).unwrap();

    // This code is from https://github.com/Smithay/wayland-rs/pull/572/files, I don't know how it works'
    let fd = connection
        .prepare_read()
        .unwrap()
        .connection_fd()
        .as_raw_fd();
    glib::source::unix_fd_add_local(fd, glib::IOCondition::IN, move |_, _| {
        connection.prepare_read().unwrap().read().unwrap();
        glib::Continue(true)
    });

    glib::MainContext::default().spawn_local(async move {
        std::future::poll_fn(|cx| event_queue.poll_dispatch_pending(cx, &mut app_data))
            .await
            .unwrap();
    });

    glib::MainLoop::new(None, false).run();
}
