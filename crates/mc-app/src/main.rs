// On Windows, a graphical application in release mode must not open a
// console in addition to its window. In debug, it should: that's where the
// logs go during development.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    mc_app_lib::run();
}
