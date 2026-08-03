use super::*;

pub(super) const TOP_BOTTOM_IO_RMUX_LOOKUP: [[IOLocalLineSource; 7]; 8] = [
    // RMUX 0
    [
        IOLocalLineSource::RoutingWire {
            ty: WireType::T4,
            bundle: 0,
            wire_idx: 0,
        },
        IOLocalLineSource::RoutingWire {
            ty: WireType::T4,
            bundle: 1,
            wire_idx: 0,
        },
        IOLocalLineSource::RoutingWire {
            ty: WireType::T4,
            bundle: 2,
            wire_idx: 0,
        },
        IOLocalLineSource::RoutingWire {
            ty: WireType::T4,
            bundle: 3,
            wire_idx: 0,
        },
        IOLocalLineSource::RoutingWire {
            ty: WireType::T4,
            bundle: 0,
            wire_idx: 1,
        },
        IOLocalLineSource::RoutingWire {
            ty: WireType::T4,
            bundle: 1,
            wire_idx: 1,
        },
        IOLocalLineSource::GlobalToLocal(0),
    ],
    // RMUX 4
    [
        IOLocalLineSource::RoutingWire {
            ty: WireType::T4,
            bundle: 2,
            wire_idx: 1,
        },
        IOLocalLineSource::RoutingWire {
            ty: WireType::T4,
            bundle: 3,
            wire_idx: 1,
        },
        IOLocalLineSource::RoutingWire {
            ty: WireType::T4,
            bundle: 0,
            wire_idx: 2,
        },
        IOLocalLineSource::RoutingWire {
            ty: WireType::T4,
            bundle: 1,
            wire_idx: 2,
        },
        IOLocalLineSource::RoutingWire {
            ty: WireType::T4,
            bundle: 2,
            wire_idx: 2,
        },
        IOLocalLineSource::RoutingWire {
            ty: WireType::T4,
            bundle: 3,
            wire_idx: 2,
        },
        IOLocalLineSource::GlobalToLocal(1),
    ],
    // RMUX 8
    [
        IOLocalLineSource::RoutingWire {
            ty: WireType::T4,
            bundle: 0,
            wire_idx: 3,
        },
        IOLocalLineSource::RoutingWire {
            ty: WireType::T4,
            bundle: 1,
            wire_idx: 3,
        },
        IOLocalLineSource::RoutingWire {
            ty: WireType::T4,
            bundle: 2,
            wire_idx: 3,
        },
        IOLocalLineSource::RoutingWire {
            ty: WireType::T4,
            bundle: 3,
            wire_idx: 3,
        },
        IOLocalLineSource::RoutingWire {
            ty: WireType::T4,
            bundle: 0,
            wire_idx: 4,
        },
        IOLocalLineSource::RoutingWire {
            ty: WireType::T4,
            bundle: 1,
            wire_idx: 4,
        },
        IOLocalLineSource::GlobalToLocal(2),
    ],
    // RMUX 12
    [
        IOLocalLineSource::RoutingWire {
            ty: WireType::T4,
            bundle: 2,
            wire_idx: 4,
        },
        IOLocalLineSource::RoutingWire {
            ty: WireType::T4,
            bundle: 3,
            wire_idx: 4,
        },
        IOLocalLineSource::RoutingWire {
            ty: WireType::T4,
            bundle: 0,
            wire_idx: 5,
        },
        IOLocalLineSource::RoutingWire {
            ty: WireType::T4,
            bundle: 1,
            wire_idx: 5,
        },
        IOLocalLineSource::RoutingWire {
            ty: WireType::T4,
            bundle: 2,
            wire_idx: 5,
        },
        IOLocalLineSource::RoutingWire {
            ty: WireType::T4,
            bundle: 3,
            wire_idx: 5,
        },
        IOLocalLineSource::GlobalToLocal(3),
    ],
    // RMUX 16
    [
        IOLocalLineSource::RoutingWire {
            ty: WireType::T4,
            bundle: 0,
            wire_idx: 6,
        },
        IOLocalLineSource::RoutingWire {
            ty: WireType::T4,
            bundle: 1,
            wire_idx: 6,
        },
        IOLocalLineSource::RoutingWire {
            ty: WireType::T4,
            bundle: 2,
            wire_idx: 6,
        },
        IOLocalLineSource::RoutingWire {
            ty: WireType::T4,
            bundle: 3,
            wire_idx: 6,
        },
        IOLocalLineSource::RoutingWire {
            ty: WireType::T4,
            bundle: 0,
            wire_idx: 7,
        },
        IOLocalLineSource::RoutingWire {
            ty: WireType::T4,
            bundle: 1,
            wire_idx: 7,
        },
        IOLocalLineSource::GlobalToLocal(4),
    ],
    // RMUX 20
    [
        IOLocalLineSource::RoutingWire {
            ty: WireType::T4,
            bundle: 2,
            wire_idx: 7,
        },
        IOLocalLineSource::RoutingWire {
            ty: WireType::T4,
            bundle: 3,
            wire_idx: 7,
        },
        IOLocalLineSource::RoutingWire {
            ty: WireType::T4,
            bundle: 0,
            wire_idx: 8,
        },
        IOLocalLineSource::RoutingWire {
            ty: WireType::T4,
            bundle: 1,
            wire_idx: 8,
        },
        IOLocalLineSource::RoutingWire {
            ty: WireType::T4,
            bundle: 2,
            wire_idx: 8,
        },
        IOLocalLineSource::RoutingWire {
            ty: WireType::T4,
            bundle: 3,
            wire_idx: 8,
        },
        IOLocalLineSource::GlobalToLocal(5),
    ],
    // RMUX 24
    [
        IOLocalLineSource::RoutingWire {
            ty: WireType::T4,
            bundle: 0,
            wire_idx: 9,
        },
        IOLocalLineSource::RoutingWire {
            ty: WireType::T4,
            bundle: 1,
            wire_idx: 9,
        },
        IOLocalLineSource::RoutingWire {
            ty: WireType::T4,
            bundle: 2,
            wire_idx: 9,
        },
        IOLocalLineSource::RoutingWire {
            ty: WireType::T4,
            bundle: 3,
            wire_idx: 9,
        },
        IOLocalLineSource::RoutingWire {
            ty: WireType::T4,
            bundle: 0,
            wire_idx: 10,
        },
        IOLocalLineSource::RoutingWire {
            ty: WireType::T4,
            bundle: 1,
            wire_idx: 10,
        },
        IOLocalLineSource::GlobalToLocal(6),
    ],
    // RMUX 28
    [
        IOLocalLineSource::RoutingWire {
            ty: WireType::T4,
            bundle: 2,
            wire_idx: 10,
        },
        IOLocalLineSource::RoutingWire {
            ty: WireType::T4,
            bundle: 3,
            wire_idx: 10,
        },
        IOLocalLineSource::RoutingWire {
            ty: WireType::T4,
            bundle: 0,
            wire_idx: 11,
        },
        IOLocalLineSource::RoutingWire {
            ty: WireType::T4,
            bundle: 1,
            wire_idx: 11,
        },
        IOLocalLineSource::RoutingWire {
            ty: WireType::T4,
            bundle: 2,
            wire_idx: 11,
        },
        IOLocalLineSource::RoutingWire {
            ty: WireType::T4,
            bundle: 3,
            wire_idx: 11,
        },
        IOLocalLineSource::GlobalToLocal(7),
    ],
];
