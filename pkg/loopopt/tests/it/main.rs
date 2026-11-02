//! The integration tests, one crate: each module is the tests of one behaviour, all through one harness
//! (`common`). One crate links the solver's library once, where a crate per file linked it 19 times, and libtest
//! runs every test of every module in parallel, where cargo ran the crates one after another (docs/BUILD.md,
//! *Tests*). One module: `cargo test --profile checks --test it plan_roll::`.

mod common;

mod cap_shapes;
mod career_order;
mod career_progress;
mod chain_rate;
mod cli;
mod owned_cards;
mod plan_alternative;
mod plan_borrowed;
mod plan_looped;
mod plan_memo;
mod plan_monotone;
mod plan_order;
mod plan_roll;
mod plan_state;
mod plan_unproven;
mod plan_value;
mod rotations;
mod sim_determinism;
mod sim_looped;
mod sim_pair_search;

/// Every file in this folder is one of the modules above: a test file left out of them would compile into nothing and
/// its tests would silently stop running.
#[test]
fn every_file_is_a_module() {
    let main = include_str!("main.rs");
    let folder = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/it");
    for entry in std::fs::read_dir(folder).expect("tests/it reads") {
        let name = entry.expect("an entry").file_name().into_string().expect("a UTF-8 name");
        let Some(module) = name.strip_suffix(".rs") else { continue };
        if module != "main" {
            assert!(main.contains(&format!("\nmod {module};\n")), "tests/it/{name} is not a module of main.rs");
        }
    }
}
