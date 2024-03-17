mod type_id;

pub(crate) use self::type_id::load_then_calculate_type_id;

pub(crate) fn prev_client_id(current: u8, count: u8) -> u8 {
    if current == 0 {
        count - 1
    } else {
        current - 1
    }
}

pub(crate) fn next_client_id(current: u8, count: u8) -> u8 {
    if current + 1 < count {
        current + 1
    } else {
        0
    }
}
