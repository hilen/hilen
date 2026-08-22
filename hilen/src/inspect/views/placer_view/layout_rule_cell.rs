use ui_proc::view;

use crate::{
    deps::refs::Weak,
    gm::{LossyConvert, color::LIGHT_GRAY},
    inspect::{
        protocol::{UIRequest, ui::ViewRepr},
        views::AnchorView,
    },
    ui::{CheckBox, Setup, TextField, UIEvent, ViewData, ViewFrame},
};

#[view(crate = crate)]
pub(crate) struct LayoutRuleCell {
    pub rule_edited: UIEvent<UIRequest>,

    view:  Weak<ViewRepr>,
    index: usize,

    #[init]
    anchor:  AnchorView,
    value:   TextField,
    enabled: CheckBox,
}

impl Setup for LayoutRuleCell {
    fn setup(self: Weak<Self>) {
        self.anchor.place().l(5).center_y().custom(move |frame| {
            let height = self.height() * 0.8;
            frame.size = (height, height).into();
        });

        self.value.steal_appearance(self.enabled);
        self.value.set_text_color(LIGHT_GRAY).set_text_size(20).integer_only();
        let selected_color = *self.value.color();
        self.value.set_selected_color(selected_color.increase_by(0.05));

        self.value.place().at_right(self.anchor, 8).w(88).relative_height(self, 0.6);
        self.value.editing_ended.val(move |val| {
            let Ok(offset) = val.parse::<f32>() else {
                let old_offset = self.view.placer.get_rules()[self.index].offset();
                self.value.set_text(LossyConvert::<i64>::lossy_convert(old_offset));
                return;
            };
            self.view.placer.edit_rule(self.index).set_offset(offset);
            self.rule_edited.trigger(self.edit_request());
        });

        self.enabled.place().at_right(self.value, 8).size(28, 28);
        self.enabled.on_change(move |on| {
            self.view.placer.edit_rule(self.index).enabled = on;
            self.rule_edited.trigger(self.edit_request());
        });
    }
}

impl LayoutRuleCell {
    pub(crate) fn set_data(mut self: Weak<Self>, view: Weak<ViewRepr>, index: usize) {
        let rule = &view.placer.get_rules()[index];

        if let Some(anchor) = rule.side() {
            self.anchor.set_anchor(anchor);
            self.value.set_text(LossyConvert::<i64>::lossy_convert(rule.offset()));
            self.enabled.set_on(rule.enabled);
        }

        self.view = view;
        self.index = index;
    }

    fn edit_request(self: Weak<Self>) -> UIRequest {
        let rules = self.view.placer.get_rules();
        let rule = &rules[self.index];

        UIRequest::EditRule {
            view_id:    self.view.id.clone(),
            rule_index: self.index,
            offset:     rule.offset(),
            enabled:    rule.enabled,
        }
    }
}
