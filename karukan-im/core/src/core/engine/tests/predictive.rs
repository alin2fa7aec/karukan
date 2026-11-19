//! Predictive dictionary lookup: readings extending the typed prefix.

use super::*;

#[test]
fn predictive_dict_candidates_carry_full_reading() {
    let mut engine = InputMethodEngine::new();
    engine.dicts.system = Some(std::sync::Arc::new(dict_from_json(
        r#"[{"reading":"わせだ","candidates":[{"surface":"早稲田","score":1000.0}]}]"#,
    )));

    let candidates = engine.lookup_dict_candidates("わせ");
    let waseda = candidates
        .iter()
        .find(|c| c.text == "早稲田")
        .expect("predictive candidate");
    assert_eq!(waseda.reading.as_deref(), Some("わせだ"));
}

#[test]
fn predictive_needs_two_typed_chars() {
    let mut engine = InputMethodEngine::new();
    engine.dicts.system = Some(std::sync::Arc::new(dict_from_json(
        r#"[{"reading":"わせだ","candidates":[{"surface":"早稲田","score":1000.0}]}]"#,
    )));

    assert!(
        engine
            .lookup_dict_candidates("わ")
            .iter()
            .all(|c| c.text != "早稲田")
    );
}

#[test]
fn exact_matches_stay_ahead_of_predictive() {
    let mut engine = InputMethodEngine::new();
    engine.dicts.system = Some(std::sync::Arc::new(dict_from_json(
        r#"[
            {"reading":"わせ","candidates":[{"surface":"和瀬","score":5000.0}]},
            {"reading":"わせだ","candidates":[{"surface":"早稲田","score":100.0}]}
        ]"#,
    )));

    let candidates = engine.lookup_dict_candidates("わせ");
    let texts: Vec<&str> = candidates.iter().map(|c| c.text.as_str()).collect();
    assert_eq!(texts, ["和瀬", "早稲田"]);
    assert_eq!(candidates[0].reading.as_deref(), Some("わせ"));
    assert_eq!(candidates[1].reading.as_deref(), Some("わせだ"));
}

/// The user's scenario: typing わせ shows every completion, but the
/// moment `d` is typed the pending narrows prediction to だ/で/ど…
/// readings — ワセリン disappears, 早稲田 stays.
#[test]
fn pending_romaji_narrows_predictive_candidates() {
    let mut engine = InputMethodEngine::new();
    engine.dicts.system = Some(std::sync::Arc::new(dict_from_json(
        r#"[
            {"reading":"わせだ","candidates":[{"surface":"早稲田","score":1000.0}]},
            {"reading":"わせりん","candidates":[{"surface":"ワセリン","score":100.0}]}
        ]"#,
    )));

    for ch in "wase".chars() {
        engine.process_key(&press(ch));
    }
    let texts: Vec<String> = engine
        .lookup_dict_candidates(&engine.input_buf.reading())
        .into_iter()
        .map(|c| c.text)
        .collect();
    assert!(texts.contains(&"早稲田".to_string()));
    assert!(texts.contains(&"ワセリン".to_string()));

    engine.process_key(&press('d'));
    assert_eq!(engine.input_buf.pending(), "d");
    let texts: Vec<String> = engine
        .lookup_dict_candidates(&engine.input_buf.reading())
        .into_iter()
        .map(|c| c.text)
        .collect();
    assert!(texts.contains(&"早稲田".to_string()));
    assert!(!texts.contains(&"ワセリン".to_string()));
}

/// A tail that cannot become kana (`yk`) suppresses prediction entirely.
#[test]
fn dead_romaji_tail_suppresses_prediction() {
    let mut engine = InputMethodEngine::new();
    engine.dicts.system = Some(std::sync::Arc::new(dict_from_json(
        r#"[{"reading":"わせだ","candidates":[{"surface":"早稲田","score":1000.0}]}]"#,
    )));

    for ch in "waseyk".chars() {
        engine.process_key(&press(ch));
    }
    assert_eq!(engine.input_buf.pending(), "yk");
    let texts: Vec<String> = engine
        .lookup_dict_candidates(&engine.input_buf.reading())
        .into_iter()
        .map(|c| c.text)
        .collect();
    assert!(!texts.contains(&"早稲田".to_string()));
}

/// The conversion list gets the full ranked predictive set (paged); the
/// composing suggestion list stays capped at 3.
#[test]
fn conversion_list_gets_all_predictive_candidates() {
    let mut engine = InputMethodEngine::new();
    engine.dicts.system = Some(std::sync::Arc::new(dict_from_json(
        r#"[
            {"reading":"わせだ","candidates":[{"surface":"早稲田","score":1000.0}]},
            {"reading":"わせだし","candidates":[{"surface":"早稲田市","score":2000.0}]},
            {"reading":"わせだだいがく","candidates":[{"surface":"早稲田大学","score":3000.0}]},
            {"reading":"わせだまえ","candidates":[{"surface":"早稲田前","score":4000.0}]},
            {"reading":"わせだえき","candidates":[{"surface":"早稲田駅","score":5000.0}]}
        ]"#,
    )));

    let conversion: Vec<String> = engine
        .build_conversion_candidates("わせ", "わせ", "", 1, ConversionLookup::FULL)
        .into_iter()
        .map(|c| c.text)
        .collect();
    for surface in ["早稲田", "早稲田市", "早稲田大学", "早稲田前", "早稲田駅"]
    {
        assert!(
            conversion.contains(&surface.to_string()),
            "missing {surface}"
        );
    }

    let suggestions = engine.lookup_dict_candidates("わせ");
    assert!(suggestions.len() <= 3);
}

/// The narrowed suggestion stays selectable: わせd + Space rebuilds the
/// conversion list with the same narrowing, so 早稲田 is in the
/// selectable candidates (and ワセリン is not).
#[test]
fn conversion_with_pending_keeps_narrowed_candidates() {
    let mut engine = InputMethodEngine::new();
    engine.dicts.system = Some(std::sync::Arc::new(dict_from_json(
        r#"[
            {"reading":"わせだ","candidates":[{"surface":"早稲田","score":1000.0}]},
            {"reading":"わせりん","candidates":[{"surface":"ワセリン","score":100.0}]}
        ]"#,
    )));

    for ch in "wased".chars() {
        engine.process_key(&press(ch));
    }
    engine.process_key(&press_key(Keysym::SPACE));

    let candidates = engine.candidates().expect("conversion candidates");
    let texts: Vec<&str> = candidates
        .candidates()
        .iter()
        .map(|c| c.text.as_str())
        .collect();
    assert!(texts.contains(&"早稲田"), "早稲田 selectable: {texts:?}");
    assert!(!texts.contains(&"ワセリン"), "ワセリン excluded: {texts:?}");

    let waseda = candidates
        .candidates()
        .iter()
        .find(|c| c.text == "早稲田")
        .unwrap();
    assert_eq!(waseda.reading.as_deref(), Some("わせだ"));
}

/// Texts of the conversion list Space opens on `romaji`.
fn conversion_texts(engine: &mut InputMethodEngine, romaji: &str) -> Vec<String> {
    for ch in romaji.chars() {
        engine.process_key(&press(ch));
    }
    engine.process_key(&press_key(Keysym::SPACE));
    engine
        .candidates()
        .expect("conversion candidates")
        .candidates()
        .iter()
        .map(|c| c.text.clone())
        .collect()
}

/// Space conversion with a large user dictionary (hundreds of thousands of
/// proper nouns): a predictive (prefix) hit is a longer word the user may
/// not have meant, so it must follow the model's conversion — otherwise
/// けいざい converts to 経済移民 instead of 経済.
#[test]
fn conversion_ranks_user_predictive_after_model() {
    let mut engine = InputMethodEngine::new();
    engine.dicts.user = Some(std::sync::Arc::new(dict_from_json(
        r#"[{"reading":"けいざいいみん","candidates":[{"surface":"経済移民","score":1000.0}]}]"#,
    )));
    seed_model_cache(&mut engine, "ケイザイ", "", &["経済"]);

    let texts = conversion_texts(&mut engine, "keizai");
    assert_eq!(&texts[..2], ["経済", "経済移民"], "{texts:?}");
}

/// An exact user-dictionary match is what the user registered for this
/// very reading, so it still outranks the model.
#[test]
fn conversion_keeps_user_exact_match_ahead_of_model() {
    let mut engine = InputMethodEngine::new();
    engine.dicts.user = Some(std::sync::Arc::new(dict_from_json(
        r#"[
            {"reading":"けいざい","candidates":[{"surface":"経財","score":1000.0}]},
            {"reading":"けいざいいみん","candidates":[{"surface":"経済移民","score":1000.0}]}
        ]"#,
    )));
    seed_model_cache(&mut engine, "ケイザイ", "", &["経済"]);

    let texts = conversion_texts(&mut engine, "keizai");
    assert_eq!(&texts[..3], ["経財", "経済", "経済移民"], "{texts:?}");
}

/// Every exact match comes before every predictive one, whichever
/// dictionary each is from: a homophone in the system dictionary is closer
/// to what was typed than a longer word in the user's.
#[test]
fn conversion_ranks_exact_matches_ahead_of_predictive_across_dictionaries() {
    let mut engine = InputMethodEngine::new();
    engine.dicts.user = Some(std::sync::Arc::new(dict_from_json(
        r#"[
            {"reading":"けいざい","candidates":[{"surface":"経財","score":1000.0}]},
            {"reading":"けいざいいみん","candidates":[{"surface":"経済移民","score":1000.0}]}
        ]"#,
    )));
    engine.dicts.system = Some(std::sync::Arc::new(dict_from_json(
        r#"[
            {"reading":"けいざい","candidates":[{"surface":"軽罪","score":1000.0}]},
            {"reading":"けいざいがく","candidates":[{"surface":"経済学","score":1000.0}]}
        ]"#,
    )));
    seed_model_cache(&mut engine, "ケイザイ", "", &["経済"]);

    let texts = conversion_texts(&mut engine, "keizai");
    assert_eq!(
        &texts[..5],
        ["経財", "経済", "軽罪", "経済移民", "経済学"],
        "{texts:?}"
    );
}

/// Engine with `prediction_key = "tab"`, わせ (exact) and わせだ (longer) in the
/// system dictionary, and 早稲田大学 learned under a longer reading.
fn prediction_tab_engine() -> InputMethodEngine {
    let mut engine = InputMethodEngine::with_config(EngineConfig {
        prediction_key: PredictionKey::Tab,
        ..EngineConfig::default()
    });
    engine.dicts.system = Some(std::sync::Arc::new(dict_from_json(
        r#"[
            {"reading":"わせ","candidates":[{"surface":"和瀬","score":5000.0}]},
            {"reading":"わせだ","candidates":[{"surface":"早稲田","score":1000.0}]}
        ]"#,
    )));
    let mut cache = karukan_engine::LearningCache::new(karukan_engine::LearningConfig::default());
    cache.record("わせだだいがく", "早稲田大学");
    engine.learning = Some(cache);
    engine
}

/// Texts of the conversion list `key` opens on `romaji`.
fn texts_after_key(engine: &mut InputMethodEngine, romaji: &str, key: Keysym) -> Vec<String> {
    for ch in romaji.chars() {
        engine.process_key(&press(ch));
    }
    engine.process_key(&press_key(key));
    engine
        .candidates()
        .expect("conversion candidates")
        .candidates()
        .iter()
        .map(|c| c.text.clone())
        .collect()
}

#[test]
fn prediction_tab_space_converts_only_what_was_typed() {
    let mut engine = prediction_tab_engine();
    let texts = texts_after_key(&mut engine, "wase", Keysym::SPACE);
    assert!(texts.contains(&"和瀬".to_string()), "{texts:?}");
    assert!(!texts.contains(&"早稲田".to_string()), "{texts:?}");
    assert!(!texts.contains(&"早稲田大学".to_string()), "{texts:?}");
}

#[test]
fn prediction_tab_tab_adds_longer_readings() {
    let mut engine = prediction_tab_engine();
    let texts = texts_after_key(&mut engine, "wase", Keysym::TAB);
    assert!(texts.contains(&"和瀬".to_string()), "{texts:?}");
    assert!(texts.contains(&"早稲田".to_string()), "{texts:?}");
    // Tab keeps the learning cache under prediction_key = "tab".
    assert!(texts.contains(&"早稲田大学".to_string()), "{texts:?}");
}

#[test]
fn skipped_prediction_keeps_exact_learning() {
    let mut engine = engine_with_learned("わせ", "和世");
    let lookup = ConversionLookup {
        prediction: PredictionLookup::Skip,
        ..ConversionLookup::FULL
    };
    let texts: Vec<String> = engine
        .build_conversion_candidates("わせ", "わせ", "", 9, lookup)
        .into_iter()
        .map(|c| c.text)
        .collect();
    assert!(texts.contains(&"和世".to_string()), "{texts:?}");
}

/// Texts of the conversion list on screen.
fn shown_texts(engine: &InputMethodEngine) -> Vec<String> {
    engine
        .candidates()
        .expect("conversion candidates")
        .candidates()
        .iter()
        .map(|c| c.text.clone())
        .collect()
}

#[test]
fn prediction_tab_ctrl_j_keeps_the_lookup_of_the_key() {
    // Space left the longer readings out; the rebuilt list must too.
    let mut engine = prediction_tab_engine();
    texts_after_key(&mut engine, "wase", Keysym::SPACE);
    engine.process_key(&press_ctrl(Keysym::KEY_J));
    let texts = shown_texts(&engine);
    assert!(!texts.contains(&"早稲田".to_string()), "{texts:?}");

    // Tab put them in; the rebuilt list keeps them.
    let mut engine = prediction_tab_engine();
    texts_after_key(&mut engine, "wase", Keysym::TAB);
    engine.process_key(&press_ctrl(Keysym::KEY_J));
    let texts = shown_texts(&engine);
    assert!(texts.contains(&"早稲田".to_string()), "{texts:?}");
}

#[test]
fn prediction_tab_deleting_learning_keeps_the_lookup_of_the_key() {
    let mut engine = prediction_tab_engine();
    engine.learning.as_mut().unwrap().record("わせ", "和世");
    let texts = texts_after_key(&mut engine, "wase", Keysym::SPACE);
    assert_eq!(texts.first().map(String::as_str), Some("和世"), "{texts:?}");

    engine.process_key(&press_ctrl(Keysym::BACKSPACE));
    let texts = shown_texts(&engine);
    assert!(!texts.contains(&"和世".to_string()), "{texts:?}");
    assert!(!texts.contains(&"早稲田".to_string()), "{texts:?}");
}

#[test]
fn filtered_view_from_composing_carries_the_space_lookup() {
    // A Tab conversion before must not leave its lookup behind for the
    // view Ctrl+T opens afterwards.
    let mut engine = prediction_tab_engine();
    texts_after_key(&mut engine, "wase", Keysym::TAB);
    engine.process_key(&press_key(Keysym::ESCAPE));
    assert!(matches!(engine.state(), InputState::Composing { .. }));

    engine.process_key(&press_ctrl(Keysym::KEY_T));
    assert_eq!(
        engine.state().conversion_lookup(),
        Some(engine.space_lookup())
    );
}

#[test]
fn prediction_tab_source_views_keep_the_longer_readings() {
    // The learning and dictionary views are searches over their source,
    // so they keep the predictive matches Space left out of its list.
    let mut engine = prediction_tab_engine();
    texts_after_key(&mut engine, "wase", Keysym::SPACE);

    engine.process_key(&press_ctrl(Keysym::KEY_T));
    assert_eq!(engine.state().filter(), Some(CandidateSource::Learning));
    let texts = shown_texts(&engine);
    assert!(texts.contains(&"早稲田大学".to_string()), "{texts:?}");

    engine.process_key(&press_ctrl(Keysym::KEY_T));
    assert_eq!(engine.state().filter(), Some(CandidateSource::Dictionary));
    let texts = shown_texts(&engine);
    assert!(texts.contains(&"早稲田".to_string()), "{texts:?}");
}
