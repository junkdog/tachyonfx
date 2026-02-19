macro_rules! ctor {
    // params + description: ctor!("Type", "name", ["p1", "p2"], "desc")
    ($type:expr, $name:expr, [$($params:expr),*], $desc:expr) => {
        CallableItem::constructor($type, $name, &[$($params),*], Some($desc))
    };
    // no params + description: ctor!("Type", "name", [], "desc")
    ($type:expr, $name:expr, [], $desc:expr) => {
        CallableItem::constructor($type, $name, &[], Some($desc))
    };
    // params, no description (existing)
    ($type:expr, $name:expr, $($params:expr),*) => {
        CallableItem::constructor($type, $name, &[$($params),*], None)
    };
    // no params, no description (existing)
    ($type:expr, $name:expr) => {
        CallableItem::constructor($type, $name, &[], None)
    };
}

macro_rules! method {
    // params + description: method!("Type", "name", ["p1", "p2"], "desc")
    ($type:expr, $name:expr, [$($params:expr),*], $desc:expr) => {
        CallableItem::instance_method($type, $name, &[$($params),*], Some($desc))
    };
    // no params + description: method!("Type", "name", [], "desc")
    ($type:expr, $name:expr, [], $desc:expr) => {
        CallableItem::instance_method($type, $name, &[], Some($desc))
    };
    // params, no description (existing)
    ($type:expr, $name:expr, $($params:expr),*) => {
        CallableItem::instance_method($type, $name, &[$($params),*], None)
    };
    // no params, no description (existing)
    ($type:expr, $name:expr) => {
        CallableItem::instance_method($type, $name, &[], None)
    };
}

pub(super) use ctor;
pub(super) use method;
