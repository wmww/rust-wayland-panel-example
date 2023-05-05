pub trait ToplevelListener {
    fn updated(&mut self, title: &str, app_id: &str);
    fn closed(&mut self);
}

pub trait ToplevelController {
    fn focus(&mut self);
    fn maximize(&mut self);
    fn close(&mut self);
}

pub trait ToplevelListListener {
    fn created(&mut self, controller: Box<dyn ToplevelController>) -> Box<dyn ToplevelListener>;
}
