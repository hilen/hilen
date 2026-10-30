//! Sets of pictures that loops of the same content share. A list of cards
//! with the same mark shows 1 set: the first loop draws it, every other
//! loop finds it here, and it goes when the last loop that holds it goes.

use std::{
    cell::RefCell,
    collections::HashMap,
    rc::{Rc, Weak},
};

use anyhow::{Result, anyhow};

use super::{flipbook::Drawing, layer::Pictures};
use crate::deps::refs::main_lock::MainLock;

/// Every set that a loop still holds, by its key.
static SETS: MainLock<HashMap<u64, Weak<SharedPictures>>> = MainLock::new();

/// 1 set of pictures, shared by the loops that hold it.
pub(super) struct SharedPictures {
    state: RefCell<State>,
}

enum State {
    Drawing(Drawing),
    Ready(Rc<Pictures>),
    Failed(String),
}

/// The set with `key`, if a loop holds it.
pub(super) fn find(key: u64) -> Option<Rc<SharedPictures>> {
    SETS.get_mut().get(&key).and_then(Weak::upgrade)
}

/// A new set of pictures that the GPU draws now.
pub(super) fn drawing(key: u64, drawing: Drawing) -> Rc<SharedPictures> {
    keep(key, State::Drawing(drawing))
}

/// A new set of pictures that is there already.
pub(super) fn ready(key: u64, pictures: Pictures) -> Rc<SharedPictures> {
    keep(key, State::Ready(Rc::new(pictures)))
}

/// Every set goes, at the stop of a hot build.
#[cfg(hot)]
pub(super) fn clear() {
    SETS.get_mut().clear();
}

fn keep(key: u64, state: State) -> Rc<SharedPictures> {
    let sets = SETS.get_mut();
    sets.retain(|_, set| set.strong_count() > 0);
    let set = Rc::new(SharedPictures {
        state: RefCell::new(state),
    });
    sets.insert(key, Rc::downgrade(&set));
    set
}

impl SharedPictures {
    /// The pictures once they are there, none while the GPU draws them.
    pub(super) fn pictures(&self) -> Result<Option<Rc<Pictures>>> {
        let mut state = self.state.borrow_mut();
        if let State::Drawing(drawing) = &*state {
            let Some(frames) = drawing.poll() else {
                return Ok(None);
            };
            *state = match frames.and_then(|frames| Pictures::new(&frames)) {
                Ok(pictures) => State::Ready(Rc::new(pictures)),
                Err(error) => State::Failed(format!("{error:#}")),
            };
        }
        match &*state {
            State::Drawing(_) => Ok(None),
            State::Ready(pictures) => Ok(Some(pictures.clone())),
            State::Failed(error) => Err(anyhow!("{error}")),
        }
    }
}
