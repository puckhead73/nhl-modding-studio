//! Real-corpus smoke test for the formula engine. Ignored by default; run against a built corpus:
//!   FHM_CORPUS=<path> cargo test -p studio-domain --test formula_real -- --ignored --nocapture

use studio_domain::{compute_person_ratings, default_profile, AttributeMaxima, FhmCorpus};

#[test]
#[ignore]
fn computes_ratings_for_a_real_player() {
    let path = std::env::var("FHM_CORPUS").expect("set FHM_CORPUS to a built corpus .db");
    let corpus = FhmCorpus::open(std::path::Path::new(&path)).expect("open corpus");

    // Find Sidney Crosby.
    let people = corpus.person_summaries("crosby", None, 50).unwrap();
    let crosby = people
        .iter()
        .find(|p| p.last_name == "Crosby" && p.first_name == "Sidney")
        .expect("Crosby present");
    let year = crosby.last_year.unwrap();

    let profile = default_profile();
    // Default profile is Nominal, which ignores the maxima.
    let cr = compute_person_ratings(
        &corpus,
        crosby.person_id,
        year,
        &profile,
        &AttributeMaxima::default(),
    )
    .unwrap()
    .expect("Crosby has a position + ratings");
    let set = &cr.set;

    println!(
        "Crosby {year}: position={} overall={:?} style={:?} fhm_attrs={}",
        set.position,
        set.overall,
        set.archetype.as_ref().map(|a| &a.ea_label),
        cr.fhm_values.len(),
    );
    for r in &set.ratings {
        println!("  {:<22} {:?}", r.ea_key, r.value);
    }

    assert_eq!(set.position, "forward");
    // A generational player should compute a high overall and most ratings should be populated.
    assert!(set.overall.unwrap() > 60, "overall {:?}", set.overall);
    let populated = set.ratings.iter().filter(|r| r.value.is_some()).count();
    assert!(populated >= 20, "only {populated} ratings populated");
}
