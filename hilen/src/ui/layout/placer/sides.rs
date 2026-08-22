// Letter order in combos doesn't matter: lrt == ltr == tlr.
// Every permutation exists on purpose so any order you type just works.

use super::Placer;
use crate::{
    gm::ToF32,
    ui::layout::{Anchor, layout_rule::LayoutRule},
};

impl Placer {
    pub fn t(&self, offset: impl ToF32) -> &Self {
        self.rules().push(LayoutRule::make(Anchor::Top, offset.to_f32()));
        self
    }

    pub fn b(&self, offset: impl ToF32) -> &Self {
        self.rules().push(LayoutRule::make(Anchor::Bot, offset.to_f32()));
        self
    }

    pub fn l(&self, offset: impl ToF32) -> &Self {
        self.rules().push(LayoutRule::make(Anchor::Left, offset.to_f32()));
        self
    }

    pub fn r(&self, offset: impl ToF32) -> &Self {
        self.rules().push(LayoutRule::make(Anchor::Right, offset.to_f32()));
        self
    }
}

impl Placer {
    pub fn tb(&self, offset: impl ToF32) -> &Self {
        self.t(offset).b(offset)
    }

    pub fn bt(&self, offset: impl ToF32) -> &Self {
        self.tb(offset)
    }

    pub fn tl(&self, offset: impl ToF32) -> &Self {
        self.t(offset).l(offset)
    }

    pub fn lt(&self, offset: impl ToF32) -> &Self {
        self.tl(offset)
    }

    pub fn tr(&self, offset: impl ToF32) -> &Self {
        self.t(offset).r(offset)
    }

    pub fn rt(&self, offset: impl ToF32) -> &Self {
        self.tr(offset)
    }

    pub fn bl(&self, offset: impl ToF32) -> &Self {
        self.b(offset).l(offset)
    }

    pub fn lb(&self, offset: impl ToF32) -> &Self {
        self.bl(offset)
    }

    pub fn br(&self, offset: impl ToF32) -> &Self {
        self.b(offset).r(offset)
    }

    pub fn rb(&self, offset: impl ToF32) -> &Self {
        self.br(offset)
    }

    pub fn lr(&self, offset: impl ToF32) -> &Self {
        self.l(offset).r(offset)
    }

    pub fn rl(&self, offset: impl ToF32) -> &Self {
        self.lr(offset)
    }
}

impl Placer {
    pub(crate) fn tlb(&self, offset: impl ToF32) -> &Self {
        self.t(offset).l(offset).b(offset)
    }

    pub fn tbl(&self, offset: impl ToF32) -> &Self {
        self.tlb(offset)
    }

    pub fn ltb(&self, offset: impl ToF32) -> &Self {
        self.tlb(offset)
    }

    pub fn lbt(&self, offset: impl ToF32) -> &Self {
        self.tlb(offset)
    }

    pub fn btl(&self, offset: impl ToF32) -> &Self {
        self.tlb(offset)
    }

    pub fn blt(&self, offset: impl ToF32) -> &Self {
        self.tlb(offset)
    }

    pub fn trb(&self, offset: impl ToF32) -> &Self {
        self.t(offset).r(offset).b(offset)
    }

    pub fn tbr(&self, offset: impl ToF32) -> &Self {
        self.trb(offset)
    }

    pub fn rtb(&self, offset: impl ToF32) -> &Self {
        self.trb(offset)
    }

    pub fn rbt(&self, offset: impl ToF32) -> &Self {
        self.trb(offset)
    }

    pub fn btr(&self, offset: impl ToF32) -> &Self {
        self.trb(offset)
    }

    pub fn brt(&self, offset: impl ToF32) -> &Self {
        self.trb(offset)
    }

    pub fn lrt(&self, offset: impl ToF32) -> &Self {
        self.l(offset).r(offset).t(offset)
    }

    pub fn ltr(&self, offset: impl ToF32) -> &Self {
        self.lrt(offset)
    }

    pub fn tlr(&self, offset: impl ToF32) -> &Self {
        self.lrt(offset)
    }

    pub fn trl(&self, offset: impl ToF32) -> &Self {
        self.lrt(offset)
    }

    pub fn rtl(&self, offset: impl ToF32) -> &Self {
        self.lrt(offset)
    }

    pub fn rlt(&self, offset: impl ToF32) -> &Self {
        self.lrt(offset)
    }

    pub fn lrb(&self, offset: impl ToF32) -> &Self {
        self.l(offset).r(offset).b(offset)
    }

    pub fn lbr(&self, offset: impl ToF32) -> &Self {
        self.lrb(offset)
    }

    pub fn blr(&self, offset: impl ToF32) -> &Self {
        self.lrb(offset)
    }

    pub fn brl(&self, offset: impl ToF32) -> &Self {
        self.lrb(offset)
    }

    pub fn rbl(&self, offset: impl ToF32) -> &Self {
        self.lrb(offset)
    }

    pub fn rlb(&self, offset: impl ToF32) -> &Self {
        self.lrb(offset)
    }
}

impl Placer {
    #[cfg(feature = "ui-tests")]
    pub(crate) fn sides(&self, sides: &str, offset: impl ToF32) -> &Self {
        for ch in sides.chars() {
            match ch {
                't' => {
                    self.t(offset);
                }
                'b' => {
                    self.b(offset);
                }
                'l' => {
                    self.l(offset);
                }
                'r' => {
                    self.r(offset);
                }
                _ => panic!("Invalid side. Use letters tblr"),
            }
        }
        self
    }

    pub fn all_sides(&self, offset: impl ToF32) -> &Self {
        self.t(offset).b(offset).l(offset).r(offset)
    }
}
