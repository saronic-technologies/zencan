//! Command-line utilities for zencan
//!
//! Collection of tools for interacting with devices via a socketcan interface on linux.
//!
//! # zencandump
//!
//! Monitors a bus, and prints each message received to stdout. Similar to the popoular `candump`
//! utility, but with some interpretation of known CANOpen messages.
//!
//! Usage example: `zencandump can0`
//!
//! # zencan-cli
//!
//! A REPL-style interactive shell for controlling CAN devices.
//!

pub mod command;
