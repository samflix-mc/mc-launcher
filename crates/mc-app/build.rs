fn main() {
    // Le nom affiché est lu à la compilation. Sans cette déclaration, le
    // changer ne déclencherait aucune recompilation et le binaire garderait
    // l'ancien — voir `src/marque.rs`.
    println!("cargo:rerun-if-env-changed=MC_LAUNCHER_NOM");
    tauri_build::build()
}
