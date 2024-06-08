#![allow(
    clippy::useless_transmute,
    clippy::transmute_int_to_bool,
    clippy::unnecessary_cast,
    clippy::too_many_arguments,
    clippy::wrong_self_convention,
    clippy::type_complexity
)]
pub mod generated {
    include!(concat!(env!("OUT_DIR"), "/vmlinux.rs"));
}
