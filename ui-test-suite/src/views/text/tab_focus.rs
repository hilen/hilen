use anyhow::Result;
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{Button, ModifiersState, NamedKey, Setup, TextField, ViewData, ViewSubviews, ViewTest, view},
    ui_test::{check_colors, inject_keys, inject_modifiers, inject_named_key},
};

const FIELDS: usize = 20;

const FOCUSED_FIRST_PROBES: &str = r"
               4    4 - #597c95
             260   20 - #bcbcbc
             576   20 - #ffffff
             480   40 - #bcbcbc
             124   88 - #bcbcbc
             396   92 - #e1e1e1
             252  164 - #ffffff
             496  176 - #bcbcbc
             108  188 - #d9d9d9
             396  188 - #e1e1e1
             168  284 - #c3c3c3
             404  288 - #ffffff
               4  328 - #597c95
             452  380 - #ffffff
             576  380 - #ffffff
             188  384 - #bcbcbc
             396  416 - #e1e1e1
             104  468 - #d8d8d8
             492  468 - #c9c9c9
             408  480 - #ffffff
              80  528 - #000000
             200  528 - #000000
             220  528 - #9a9a9a
             220  532 - #9a9a9a
             124  536 - #7a7a7a
             124  540 - #7a7a7a
             176  540 - #000000
             220  540 - #9a9a9a
              96  544 - #000000
             140  544 - #010101
             212  548 - #010101
             592  592 - #597c95
";

const FILLED_PROBES: &str = r"
             276   20 - #bcbcbc
             576   20 - #ffffff
              20   24 - #bcbcbc
             444   28 - #181818
             156   40 - #797979
             148   44 - #bcbcbc
             444   44 - #181818
             144   92 - #000000
             448   92 - #8c8c8c
             448  140 - #8c8c8c
             156  172 - #424242
             156  184 - #424242
             456  184 - #565656
             452  188 - #ffffff
             444  236 - #010101
             152  272 - #080808
             448  284 - #ffffff
             456  324 - #585858
             156  328 - #303030
             456  340 - #585858
             156  376 - #505050
             320  388 - #ffffff
             448  420 - #ffffff
             148  424 - #8c8c8c
             148  468 - #747474
             448  472 - #000000
             148  480 - #747474
             220  528 - #9a9a9a
             108  532 - #000000
             168  544 - #cacaca
             328  592 - #597c95
             592  592 - #597c95
";

const FINAL_PROBES: &str = r"
             592    4 - #597c95
             444   28 - #181818
             156   40 - #a4a4a4
             448   80 - #8c8c8c
             144   92 - #000000
             148  132 - #ffffff
             448  136 - #8c8c8c
             156  172 - #424242
              20  184 - #ffffff
             456  184 - #565656
             156  188 - #424242
             444  228 - #010101
             144  232 - #000000
             148  284 - #101010
             448  284 - #ffffff
             456  324 - #585858
             156  328 - #303030
             456  340 - #585858
              20  360 - #ffffff
             156  376 - #505050
             448  420 - #ffffff
             148  424 - #8c8c8c
             576  452 - #bcbcbc
             320  456 - #bcbcbc
             148  468 - #747474
             148  472 - #747474
             460  476 - #bcbcbc
             148  480 - #747474
             108  532 - #000000
             168  540 - #cacaca
             220  544 - #9a9a9a
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
        inject_modifiers(ModifiersState::SHIFT);
        inject_named_key(NamedKey::Tab);
        inject_modifiers(ModifiersState::empty());
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
