use super::toplevels::*;
use slotmap::{new_key_type, SlotMap};
use std::{cell::RefCell, rc::Rc, sync::Mutex};
use wayland_client::{
    event_created_child,
    globals::GlobalListContents,
    protocol::{
        wl_registry::{Event, WlRegistry},
        wl_seat::WlSeat,
    },
    Connection, Dispatch, EventQueue, Proxy, QueueHandle,
};
use wayland_protocols_wlr::foreign_toplevel::v1::client::{
    zwlr_foreign_toplevel_handle_v1::{self, ZwlrForeignToplevelHandleV1},
    zwlr_foreign_toplevel_manager_v1::{self, ZwlrForeignToplevelManagerV1},
};

new_key_type! { struct ToplevelKey; }

struct ToplevelData {
    title: String,
    app_id: String,
    listener: Option<Box<dyn ToplevelListener>>,
    handle: ZwlrForeignToplevelHandleV1,
}

impl ToplevelData {
    fn new(handle: ZwlrForeignToplevelHandleV1) -> Self {
        ToplevelData {
            title: "".to_string(),
            app_id: "".to_string(),
            listener: None,
            handle,
        }
    }
}

struct WaylandData {
    _toplevel_manager: ZwlrForeignToplevelManagerV1,
    seat: WlSeat,
    toplevel_listener: Box<dyn ToplevelListListener>,
    toplevels: SlotMap<ToplevelKey, ToplevelData>,
}

#[derive(Clone)]
pub struct WaylandState(Rc<RefCell<WaylandData>>);

struct ToplevelControllerImpl {
    key: ToplevelKey,
    state: WaylandState,
}

impl ToplevelController for ToplevelControllerImpl {
    fn focus(&mut self) {
        let mut data = self.state.0.borrow_mut();
        let data = &mut *data;
        let toplevel = data.toplevels.get_mut(self.key).unwrap();
        toplevel.handle.activate(&data.seat);
    }

    fn maximize(&mut self) {
        let mut data = self.state.0.borrow_mut();
        let toplevel = data.toplevels.get_mut(self.key).unwrap();
        toplevel.handle.set_maximized();
    }

    fn close(&mut self) {
        let mut data = self.state.0.borrow_mut();
        let toplevel = data.toplevels.get_mut(self.key).unwrap();
        toplevel.handle.close();
    }
}

pub fn init(
    connection: &Connection,
    seat: WlSeat,
    toplevel_listener: Box<dyn ToplevelListListener>,
) -> (EventQueue<WaylandState>, WaylandState) {
    let (globals, event_queue) =
        wayland_client::globals::registry_queue_init::<WaylandState>(&connection).unwrap();
    let qh = event_queue.handle();
    let _toplevel_manager = globals
        .bind::<ZwlrForeignToplevelManagerV1, _, _>(&qh, core::ops::RangeInclusive::new(1, 1), ())
        .unwrap();
    let data = WaylandData {
        _toplevel_manager,
        seat,
        toplevel_listener,
        toplevels: SlotMap::with_key(),
    };
    (event_queue, WaylandState(Rc::new(RefCell::new(data))))
}

impl Dispatch<WlRegistry, GlobalListContents> for WaylandState {
    fn event(
        _: &mut Self,
        _: &WlRegistry,
        _: Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<WaylandState>,
    ) {
    }
}

impl Dispatch<zwlr_foreign_toplevel_manager_v1::ZwlrForeignToplevelManagerV1, ()> for WaylandState {
    fn event(
        state: &mut Self,
        _: &ZwlrForeignToplevelManagerV1,
        event: zwlr_foreign_toplevel_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<WaylandState>,
    ) {
        match event {
            zwlr_foreign_toplevel_manager_v1::Event::Toplevel { toplevel } => {
                let toplevel_data = ToplevelData::new(toplevel.clone());
                let mut wayland_data = state.0.borrow_mut();
                let toplevel_key = wayland_data.toplevels.insert(toplevel_data);
                let controller_impl = ToplevelControllerImpl {
                    key: toplevel_key,
                    state: state.clone(),
                };
                let listener = wayland_data
                    .toplevel_listener
                    .created(Box::new(controller_impl));
                wayland_data
                    .toplevels
                    .get_mut(toplevel_key)
                    .unwrap()
                    .listener = Some(listener);
                let user_data: &Mutex<Option<ToplevelKey>> = toplevel.data().unwrap();
                *user_data.lock().unwrap() = Some(toplevel_key);
            }
            _ => (),
        }
    }

    event_created_child!(Self, ZwlrForeignToplevelHandleV1, [
        zwlr_foreign_toplevel_manager_v1::EVT_TOPLEVEL_OPCODE => (ZwlrForeignToplevelHandleV1, Mutex::new(None))
    ]);
}

impl Dispatch<ZwlrForeignToplevelHandleV1, Mutex<Option<ToplevelKey>>> for WaylandState {
    fn event(
        state: &mut Self,
        _: &ZwlrForeignToplevelHandleV1,
        event: zwlr_foreign_toplevel_handle_v1::Event,
        key: &Mutex<Option<ToplevelKey>>,
        _: &wayland_client::Connection,
        _: &wayland_client::QueueHandle<WaylandState>,
    ) {
        match event {
            zwlr_foreign_toplevel_handle_v1::Event::Title { title } => {
                let mut data = state.0.borrow_mut();
                let toplevel = data
                    .toplevels
                    .get_mut(key.lock().unwrap().unwrap())
                    .unwrap();
                toplevel.title = title;
            }
            zwlr_foreign_toplevel_handle_v1::Event::AppId { app_id } => {
                let mut data = state.0.borrow_mut();
                let toplevel = data
                    .toplevels
                    .get_mut(key.lock().unwrap().unwrap())
                    .unwrap();
                toplevel.app_id = app_id;
            }
            zwlr_foreign_toplevel_handle_v1::Event::Done {} => {
                let mut data = state.0.borrow_mut();
                let toplevel = data
                    .toplevels
                    .get_mut(key.lock().unwrap().unwrap())
                    .unwrap();
                toplevel
                    .listener
                    .as_mut()
                    .unwrap()
                    .updated(&toplevel.title, &toplevel.app_id);
            }
            zwlr_foreign_toplevel_handle_v1::Event::Closed {} => {
                let mut data = state.0.borrow_mut();
                let toplevel = data
                    .toplevels
                    .get_mut(key.lock().unwrap().unwrap())
                    .unwrap();
                toplevel.listener.as_mut().unwrap().closed();
            }
            _ => (),
        }
    }
}
