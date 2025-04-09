use std::collections::{HashMap, HashSet};

#[derive(Clone)]
pub(crate) struct Breakpoints {
    pub(crate) vblank: bool,
    pub(crate) hblank: bool,
    pub(crate) interrupt: bool,
    pub(crate) breakpoints: HashSet<u16>,
    pub(crate) watchpoints: HashMap<u16, u8>,
}

impl Breakpoints {
    pub(crate) fn new() -> Breakpoints {
        Breakpoints {
            vblank: false,
            hblank: false,
            interrupt: false,
            breakpoints: HashSet::new(),
            watchpoints: HashMap::new(),
        }
    }
}
