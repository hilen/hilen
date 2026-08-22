use std::fmt;

use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{self, MapAccess, Visitor},
    ser::SerializeStruct,
};

use crate::{deps::refs::Rglica, gm::RefCell, ui::Placer};

impl Serialize for Placer {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        let mut state = serializer.serialize_struct("Placer", 5)?;

        // Use .borrow() to access the inner value of RefCells
        state.serialize_field("rules", &*self.rules.borrow())?;
        state.serialize_field("all_tiling_rules", &*self.all_tiling_rules.borrow())?;
        // state.serialize_field("view", &self.view)?;
        // state.serialize_field("s_content", &self.s_content)?;
        state.serialize_field("all_margin", &*self.all_margin.borrow())?;
        state.serialize_field("has", &*self.has.borrow())?;
        state.serialize_field("fit_text", &*self.fit_text.borrow())?;

        state.end()
    }
}

impl<'de> Deserialize<'de> for Placer {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        // Define the expected field names
        #[allow(non_camel_case_types)]
        enum Field {
            rules,
            all_tiling_rules,
            // view,
            // s_content,
            all_margin,
            has,
            fit_text,
        }

        impl<'de> Deserialize<'de> for Field {
            fn deserialize<D>(deserializer: D) -> Result<Field, D::Error>
            where D: Deserializer<'de> {
                struct FieldVisitor;
                impl Visitor<'_> for FieldVisitor {
                    type Value = Field;
                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("field identifier")
                    }
                    fn visit_str<E>(self, value: &str) -> Result<Field, E>
                    where E: de::Error {
                        match value {
                            "rules" => Ok(Field::rules),
                            "all_tiling_rules" => Ok(Field::all_tiling_rules),
                            // "view" => Ok(Field::view),
                            // "s_content" => Ok(Field::s_content),
                            "all_margin" => Ok(Field::all_margin),
                            "has" => Ok(Field::has),
                            "fit_text" => Ok(Field::fit_text),
                            _ => Err(de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(FieldVisitor)
            }
        }

        struct PlacerVisitor;
        impl<'de> Visitor<'de> for PlacerVisitor {
            type Value = Placer;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct Placer")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Placer, V::Error>
            where V: MapAccess<'de> {
                let mut rules = None;
                let mut all_tiling_rules = None;
                // let mut view = None;
                // let mut s_content = None;
                let mut all_margin = None;
                let mut has = None;
                let mut fit_text = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::rules => rules = Some(map.next_value()?),
                        Field::all_tiling_rules => all_tiling_rules = Some(map.next_value()?),
                        // Field::view => view = Some(map.next_value()?),
                        // Field::s_content => s_content = Some(map.next_value()?),
                        Field::all_margin => all_margin = Some(map.next_value()?),
                        Field::has => has = Some(map.next_value()?),
                        Field::fit_text => fit_text = Some(map.next_value()?),
                    }
                }

                Ok(Placer {
                    rules:            RefCell::new(rules.ok_or_else(|| de::Error::missing_field("rules"))?),
                    all_tiling_rules: RefCell::new(
                        all_tiling_rules.ok_or_else(|| de::Error::missing_field("all_tiling_rules"))?,
                    ),
                    view:             Rglica::default(), /* view.ok_or_else(||
                                                          * de::Error::missing_field("view"))?, */
                    s_content:        Rglica::default(), /* s_content.ok_or_else(||
                                                          * de::Error::missing_field("s_content"))?, */
                    all_margin:       RefCell::new(
                        all_margin.ok_or_else(|| de::Error::missing_field("all_margin"))?,
                    ),
                    has:              RefCell::new(has.ok_or_else(|| de::Error::missing_field("has"))?),
                    fit_text:         RefCell::new(
                        fit_text.ok_or_else(|| de::Error::missing_field("fit_text"))?,
                    ),
                    custom:           RefCell::new(None),
                })
            }
        }

        const FIELDS: &[&str] = &[
            "rules",
            "all_tiling_rules",
            "view",
            "s_content",
            "all_margin",
            "has",
            "fit_text",
        ];
        deserializer.deserialize_struct("Placer", FIELDS, PlacerVisitor)
    }
}
