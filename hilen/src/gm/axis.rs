use std::marker::ConstParamTy;

#[derive(Copy, Clone, PartialEq, Eq, ConstParamTy)]
pub(crate) enum Axis {
    X,
    Y,
}
