// Sous Windows, une application graphique en release ne doit pas ouvrir de
// console en plus de sa fenêtre. En debug, si : c'est là que passent les
// journaux pendant le développement.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    samflix_launcher_lib::run();
}
