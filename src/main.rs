mod toplevels;
mod wayland;

use gtk4::{glib, prelude::*, Button, IconTheme, Orientation, Widget};
use std::{os::fd::AsRawFd, rc::Rc};
use toplevels::{ToplevelController, ToplevelListListener, ToplevelListener};
use wayland_client::Proxy;

pub struct Taskbar {
    icon_theme: IconTheme,
    root: gtk4::Box,
}

impl Taskbar {
    pub fn new(icon_theme: IconTheme) -> Rc<Self> {
        let root = gtk4::Box::new(Orientation::Vertical, 5);
        Rc::new(Self { icon_theme, root })
    }

    pub fn widget(&self) -> &Widget {
        self.root.upcast_ref()
    }
}

impl ToplevelListListener for Rc<Taskbar> {
    fn created(&mut self, controller: Box<dyn ToplevelController>) -> Box<dyn ToplevelListener> {
        let button = Button::builder().build();
        let controller = std::cell::RefCell::new(controller);
        button.connect_clicked(move |_button| {
            controller.borrow_mut().focus();
        });
        self.root.append(&button);
        Box::new(TaskbarItem {
            taskbar: self.clone(),
            button,
        })
    }
}

struct TaskbarItem {
    taskbar: Rc<Taskbar>,
    button: Button,
}

impl ToplevelListener for TaskbarItem {
    fn updated(&mut self, title: &str, app_id: &str) {
        //self.button.set_icon_name(app_id);
        self.button.set_tooltip_text(Some(title));
        let icon = self.taskbar.icon_theme.lookup_icon(
            app_id,
            &[],
            32,
            1,
            gtk4::TextDirection::Ltr,
            gtk4::IconLookupFlags::empty(),
        );
        let picture = gtk4::Picture::for_paintable(&icon);
        self.button.set_child(Some(&picture));
    }
    fn closed(&mut self) {}
}

fn main() {
    gtk4::init().unwrap();

    let gdk_display = gtk4::gdk::Display::default().unwrap();
    let gdk_wayland_display = gdk_display
        .clone()
        .downcast::<gdk4_wayland::WaylandDisplay>()
        .unwrap();
    let wl_display = gdk_wayland_display.wl_display().unwrap();
    let wl_display_ptr = wl_display.id().as_ptr();
    let connection = wayland_client::Connection::from_backend(unsafe {
        wayland_client::backend::Backend::from_foreign_display(wl_display_ptr as _)
    });
    let gdk_wayland_seat = gdk_display
        .default_seat()
        .unwrap()
        .downcast::<gdk4_wayland::WaylandSeat>()
        .unwrap();
    let wl_seat = gdk_wayland_seat.wl_seat().unwrap();

    let window = gtk4::Window::new();
    let icon_theme = IconTheme::for_display(&gdk_display);
    let taskbar = Taskbar::new(icon_theme);
    window.set_child(Some(taskbar.widget()));
    gtk4_layer_shell::init_for_window(&window);
    gtk4_layer_shell::set_anchor(&window, gtk4_layer_shell::Edge::Left, true);
    //gtk4_layer_shell::set_anchor(&window, gtk4_layer_shell::Edge::Top, true);
    //gtk4_layer_shell::set_anchor(&window, gtk4_layer_shell::Edge::Bottom, true);
    gtk4_layer_shell::auto_exclusive_zone_enable(&window);
    window.show();

    let (mut event_queue, mut state) = wayland::init(&connection, wl_seat, Box::new(taskbar));

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
        std::future::poll_fn(|cx| event_queue.poll_dispatch_pending(cx, &mut state))
            .await
            .unwrap();
    });

    glib::MainLoop::new(None, false).run();
}
