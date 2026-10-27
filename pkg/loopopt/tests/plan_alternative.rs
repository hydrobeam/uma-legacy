//! An Alternative side is lifted on its own lineage (docs/SOLVER.md, "Next career"): on the val/plan p10 pool V's trio
//! takes Agnes Digital's Alternative side on [6, 7] (another of her careers, where she builds), and V's route's first
//! step is her builder, El Condor Pasa [6, 7]. That row's score goes through every part of the lift: the side's own
//! parents beside the builder's veteran (`own_parent_places`), its own deck and record (`lift_side`), the lift reads
//! that bound and prune its routes (`lift_reads`), and its routes. It saves about one career, as V's route promises
//! (planval's first-step tripwire, -0.1); with no lift of an Alternative side it scored -0.585 (c89918a's admission
//! with no lift), so a broken own-parent construction, or reads that prune its routes, fail it.
use serde_json::Value;
use std::process::Command;

#[test]
fn an_alternative_sides_builder_saves_about_one_career() {
    let job = concat!(env!("CARGO_MANIFEST_DIR"), "/../../val/plan/p10.json");
    let output = Command::new(env!("CARGO_BIN_EXE_loopopt"))
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/data/global"))
        .arg("data.json")
        .arg(job)
        .env_remove("LOOPOPT_STATE_IN")
        .env_remove("LOOPOPT_STATE_OUT")
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let text = String::from_utf8_lossy(&output.stdout);
    let last = text.lines().rev().find(|line| !line.trim().is_empty()).unwrap();
    let plan = &serde_json::from_str::<Value>(last).unwrap()["next"];
    let members = plan["members"].as_array().unwrap();
    let alternative = members
        .iter()
        .find(|member| member["alternative"] == true)
        .unwrap_or_else(|| panic!("V's trio takes an Alternative side: {}", plan["members"]));
    assert!(alternative["building_careers"].as_f64().unwrap() > 0.0, "and she builds there: {alternative}");
    let builder = alternative["builder"].as_str().expect("her Cont's builder");
    let row = plan["options"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["career"] == builder)
        .unwrap_or_else(|| panic!("a row of her builder {builder}"));
    let score = row["score"].as_f64().unwrap();
    assert!(score >= -0.1, "V's route's first step saves {} careers, against the 1 it promises", score + 1.0);
    assert_eq!(plan["routes_dropped_uncertified"], 0.0, "{}", plan["routes_dropped_uncertified"]);
}
