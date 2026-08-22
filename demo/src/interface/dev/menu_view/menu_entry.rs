use hilen::net::Function;

use crate::interface::dev::Node;

#[derive(Default, Clone, Debug)]
pub struct MenuEntry {
    pub label: &'static str,
    action:    Function<(), ()>,
}

impl MenuEntry {
    pub fn new(label: &'static str) -> Self {
        Self {
            label,
            action: Function::default(),
        }
    }

    pub fn action<Ret>(mut self, mut action: impl FnMut() -> Ret + Send + 'static) -> Self {
        self.action = Function::new(move |()| {
            action();
        });
        self
    }

    pub fn run(&self) {
        self.action.call(());
    }
}

impl From<MenuEntry> for Node<MenuEntry> {
    fn from(value: MenuEntry) -> Self {
        Self::empty(value)
    }
}
