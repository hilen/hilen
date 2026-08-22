use educe::Educe;

use crate::gm::num::into_f32::ToF32;

#[derive(Debug, Educe)]
#[educe(Default)]
pub struct Converter {
    #[educe(Default = 0.0)]
    min:  f32,
    #[educe(Default = 1.0)]
    max:  f32,
    #[educe(Default = 1.0)]
    span: f32,
}

impl Converter {
    pub fn new(min: impl ToF32, max: impl ToF32) -> Self {
        let min = min.to_f32();
        let max = max.to_f32();
        Self {
            min,
            max,
            span: max - min,
        }
    }

    pub fn set_min(&mut self, min: impl ToF32) -> &mut Self {
        self.min = min.to_f32();
        self.span = self.max - self.min;
        self
    }

    pub(crate) fn set_max(&mut self, max: impl ToF32) -> &mut Self {
        self.max = max.to_f32();
        self.span = self.max - self.min;
        self
    }

    pub(crate) fn convert(&self, val: impl ToF32) -> f32 {
        self.min + val.to_f32() * self.span
    }

    pub(crate) fn reverse_convert(&self, val: impl ToF32) -> f32 {
        (val.to_f32() - self.min) / self.span
    }
}

// #[cfg(test)]
// mod test {
//     use fake::Fake;
//
//     use crate::gm::converter::Converter;
//
//     #[test]
//     fn test_converter() {
//         for (min, max) in [
//             (-5.0, 10.0),
//             (-100000.0, 1000000.0),
//             (0.00001, 0.00004),
//             (100.0, 500.0),
//             (0.0, 1.0),
//             (5000.0, 2.0),
//             (50543500.0, -1000.0),
//         ] {
//             let conv = Converter::new(min, max);
//
//             let mut edited_conv = Converter::default();
//
//             edited_conv.set_max(max);
//             edited_conv.set_min(min);
//
//             assert_eq!(conv.convert(0), min);
//             assert_eq!(conv.convert(1), max);
//
//             assert_eq!(conv.reverse_convert(min), 0.0);
//             assert_eq!(conv.reverse_convert(max), 1.0);
//
//             assert_eq!(edited_conv.convert(0), min);
//             assert_eq!(edited_conv.convert(1), max);
//
//             assert_eq!(edited_conv.reverse_convert(min), 0.0);
//             assert_eq!(edited_conv.reverse_convert(max), 1.0);
//
//             for _ in 0..100 {
//                 let val = (-5.0..5.0).fake::<f32>();
//                 assert_eq!(conv.convert(val), min + val * (max - min));
//                 assert_eq!(edited_conv.convert(val), min + val * (max -
// min));
//
//                 let converted = conv.convert(val);
//                 let rev = conv.reverse_convert(converted);
//
//                 assert!((rev - val).abs() < 0.000001);
//             }
//         }
//     }
// }
