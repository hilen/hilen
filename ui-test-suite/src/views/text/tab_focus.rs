use anyhow::Result;
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{Button, Modifiers, NamedKey, Setup, TextField, ViewData, ViewSubviews, ViewTest, view},
    ui_test::{check_colors, inject_keys, inject_modifiers, inject_named_key},
};

const FIELDS: usize = 20;

const FOCUSED_FIRST_PROBES: &str = r"
             292    4 - #597c95
             576   20 - #ffffff
             124   40 - #bcbcbc
             500   76 - #bcbcbc
             396   92 - #e0e0e0
             192   96 - #c3c3c3
             112  172 - #d6d6d6
             496  176 - #bcbcbc
             396  188 - #e0e0e0
             168  268 - #c5c5c5
             492  272 - #bcbcbc
             404  288 - #ffffff
               4  324 - #597c95
             168  380 - #c5c5c5
             452  380 - #ffffff
             576  380 - #ffffff
             320  384 - #ffffff
             396  416 - #e0e0e0
             104  468 - #d6d6d6
             408  480 - #ffffff
              80  528 - #000000
             200  528 - #000000
             220  528 - #a2a2a2
             168  532 - #000000
             220  532 - #a2a2a2
             124  536 - #7a7a7a
             124  540 - #7a7a7a
             220  540 - #a2a2a2
              96  544 - #010101
             188  548 - #010101
             212  548 - #000000
             592  592 - #597c95
";

const FILLED_PROBES: &str = r"
             576   20 - #ffffff
             444   28 - #1b1b1b
             444   36 - #1b1b1b
             156   40 - #a3a3a3
             448   76 - #8a8a8a
             144   92 - #000000
             448   92 - #8a8a8a
              20  100 - #ffffff
             448  136 - #949494
             460  140 - #000000
             156  172 - #3c3c3c
             456  184 - #4d4d4d
             156  188 - #3c3c3c
             452  188 - #ffffff
             300  232 - #597c95
             148  276 - #070707
             148  284 - #070707
             448  284 - #ffffff
             456  324 - #5a5a5a
             156  328 - #383838
             456  340 - #5a5a5a
             156  380 - #4d4d4d
             148  424 - #8a8a8a
             448  464 - #000000
             148  472 - #7b7b7b
             220  528 - #a2a2a2
              80  536 - #000000
             168  536 - #d6d6d6
             168  540 - #d6d6d6
             168  544 - #d6d6d6
             220  544 - #a2a2a2
             592  592 - #597c95
";

const FINAL_PROBES: &str = r"
             576   20 - #ffffff
             444   28 - #1b1b1b
             156   40 - #a3a3a3
             444   44 - #1b1b1b
             448   76 - #8a8a8a
             144   92 - #000000
             448   92 - #8a8a8a
              20  100 - #ffffff
             448  136 - #949494
             460  140 - #000000
             156  172 - #3c3c3c
             456  184 - #4d4d4d
             156  188 - #3c3c3c
             452  188 - #ffffff
             300  232 - #597c95
             148  276 - #070707
             448  284 - #ffffff
             456  324 - #5a5a5a
             156  328 - #383838
             456  340 - #5a5a5a
             156  380 - #4d4d4d
             148  424 - #8a8a8a
             440  464 - #000000
             148  472 - #7b7b7b
             460  480 - #1a1a1a
             220  528 - #a2a2a2
              80  536 - #000000
             168  536 - #d6d6d6
             168  540 - #d6d6d6
             168  544 - #d6d6d6
             220  544 - #a2a2a2
             592  592 - #597c95
";

/// Tab moves editing to the next text field in view tree order, wrapping
/// at the ends, Shift+Tab goes backward. A button and a hidden field sit
/// between the fields and are skipped.
#[view]
struct TabFocus {
    fields: Vec<Weak<TextField>>,
    hidden: Weak<TextField>,
}

impl Setup for TabFocus {
    fn setup(mut self: Weak<Self>) {
        for i in 0..FIELDS {
            // The button after the first column and the hidden field
            // after it interleave with the fields in creation order.
            if i == 10 {
                let button = self.add_view::<Button>();
                button.set_text("Not a field");
                button.place().t(520).l(20).size(260, 36);

                let hidden = self.add_view::<TextField>();
                hidden.set_hidden(true);
                hidden.place().t(520).l(320).size(260, 36);
                self.hidden = hidden;
            }

            let field = self.add_view::<TextField>();
            field.set_placeholder(format!("Field {}", i + 1));
            let column = if i < 10 { 20 } else { 320 };
            field.place().t(20 + (i % 10) * 48).l(column).size(260, 36);
            self.fields.push(field);
        }
    }
}

impl ViewTest for TabFocus {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        from_main(move || view.fields[0].focus());

        check_colors(FOCUSED_FIRST_PROBES)?;

        // One letter per field, Tab to the next, the last Tab wraps to
        // the first field.
        for i in 0..FIELDS {
            inject_keys(char::from(b'a' + u8::try_from(i)?));
            inject_named_key(NamedKey::Tab);
        }

        check_colors(FILLED_PROBES)?;

        // Backward from the first field wraps to the last.
        inject_modifiers(Modifiers::SHIFT);
        inject_named_key(NamedKey::Tab);
        inject_modifiers(Modifiers::empty());
        inject_keys("z");

        check_colors(FINAL_PROBES)?;

        from_main(move || {
            for (i, field) in view.fields.iter().enumerate() {
                let letter = char::from(b'a' + u8::try_from(i).unwrap());
                let expected = if i == FIELDS - 1 {
                    format!("{letter}z")
                } else {
                    letter.to_string()
                };
                assert_eq!(field.text(), expected);
            }
            assert_eq!(view.hidden.text(), "");
        });

        Ok(())
    }
}
