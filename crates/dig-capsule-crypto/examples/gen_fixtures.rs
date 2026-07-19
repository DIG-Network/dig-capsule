//! Regenerates the committed fixture files under `tests/fixtures/`.
//! Run with: cargo run -p dig-capsule-crypto --example gen_fixtures

use std::path::Path;

fn main() -> std::io::Result<()> {
    let base = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures");
    dig_capsule_crypto::write_kdf_fixtures(&base.join("kdf_kat.json"))?;
    dig_capsule_crypto::write_bls_fixtures(&base.join("bls_vectors.json"))?;
    println!("wrote {}", base.join("kdf_kat.json").display());
    println!("wrote {}", base.join("bls_vectors.json").display());
    Ok(())
}
