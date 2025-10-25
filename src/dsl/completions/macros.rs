macro_rules! ctor {
    ($type:expr, $name:expr, $($params:expr),*) => {
        CallableItem::constructor($type, $name, &[$($params),*])
    };
    ($type:expr, $name:expr) => {
        CallableItem::constructor($type, $name, &[])
    };
}

macro_rules! method {
    ($type:expr, $name:expr, $($params:expr),*) => {
        CallableItem::instance_method($type, $name, &[$($params),*])
    };
    ($type:expr, $name:expr) => {
        CallableItem::instance_method($type, $name, &[])
    };
}

pub(super) use ctor;
pub(super) use method;
