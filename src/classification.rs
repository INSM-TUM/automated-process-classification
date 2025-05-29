use crate::dependency_types::{
    dependency::Dependency, existential::DependencyType as ExistentialEnum,
    temporal::DependencyType as TemporalEnum,
};
use serde::Serialize;
use std::collections::{HashMap, HashSet};

pub type Activity = String;
pub type InputMatrix = HashMap<(Activity, Activity), Dependency>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Classification {
    Structured,
    SemiStructured,
    LooselyStructured,
    StructuredSemiStructured,
    SemiStructuredLooselyStructured,
    Unstructured,
    Error(String),
}

impl std::fmt::Display for Classification {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Classification::Structured => write!(f, "Structured"),
            Classification::SemiStructured => write!(f, "Semi-Structured"),
            Classification::LooselyStructured => write!(f, "Loosely Structured"),
            Classification::StructuredSemiStructured => write!(f, "Structured / Semi-Structured"),
            Classification::SemiStructuredLooselyStructured => {
                write!(f, "Semi-Structured / Loosely Structured")
            }
            Classification::Unstructured => write!(f, "Unstructured"),
            Classification::Error(s) => write!(f, "Error in classification: {}", s),
        }
    }
}

#[derive(Debug, Default, Serialize)]
pub struct CalculatedPercentages {
    // Primary Rule related percentages
    none_none: f64,
    none_implication: f64,
    none_equivalence: f64,
    eventual_equivalence: f64,
    eventual_implication: f64,

    // Secondary Rule specific percentages
    none_negated_equivalence: f64,

    // Unstructured Rule related percentages
    eventual_any_existential: f64, // Any existential with Eventual temporal
    direct_any_existential: f64, // Any existential with Direct temporal (for completeness if needed in the future)
    direct_none: f64,
}

impl CalculatedPercentages {
    pub fn new(matrix: &InputMatrix) -> Result<Self, String> {
        if matrix.is_empty() {
            return Err("Input matrix is empty".to_string());
        }

        let total_entries = matrix.len();
        let mut counts_none_none = 0;
        let mut counts_none_implication = 0;
        let mut counts_none_equivalence = 0;
        let mut counts_none_negated_equivalence = 0;
        let mut counts_eventual_equivalence = 0;
        let mut counts_eventual_implication = 0;
        let mut counts_eventual_any = 0;
        let mut counts_direct_any = 0; // For direct_any_existential
        let mut counts_direct_none = 0;

        for dependency_obj in matrix.values() {
            let temporal_type = dependency_obj
                .temporal_dependency
                .as_ref()
                .map(|td| td.dependency_type);
            let existential_type = dependency_obj
                .existential_dependency
                .as_ref()
                .map(|ed| ed.dependency_type);

            match temporal_type {
                None => {
                    // No temporal dependency
                    match existential_type {
                        None => counts_none_none += 1,
                        Some(ExistentialEnum::Implication) => counts_none_implication += 1,
                        Some(ExistentialEnum::Equivalence) => counts_none_equivalence += 1,
                        Some(ExistentialEnum::NegatedEquivalence) => {
                            counts_none_negated_equivalence += 1
                        }
                        Some(ExistentialEnum::Nand) | Some(ExistentialEnum::Or) => {
                            // Not consider at the moment
                        }
                    }
                }
                Some(TemporalEnum::Eventual) => {
                    if existential_type.is_some() {
                        // Any existential with Eventual temporal
                        counts_eventual_any += 1;
                    }
                    match existential_type {
                        Some(ExistentialEnum::Equivalence) => counts_eventual_equivalence += 1,
                        Some(ExistentialEnum::Implication) => counts_eventual_implication += 1,
                        _ => {}
                    }
                }
                Some(TemporalEnum::Direct) => {
                    if existential_type.is_some() {
                        // Any existential with Direct temporal
                        counts_direct_any += 1;
                    } else {
                        // Direct with no existential
                        counts_direct_none += 1;
                    }
                }
            }
        }

        let total_f = total_entries as f64;
        Ok(Self {
            none_none: counts_none_none as f64 / total_f,
            none_implication: counts_none_implication as f64 / total_f,
            none_equivalence: counts_none_equivalence as f64 / total_f,
            eventual_equivalence: counts_eventual_equivalence as f64 / total_f,
            eventual_implication: counts_eventual_implication as f64 / total_f,
            none_negated_equivalence: counts_none_negated_equivalence as f64 / total_f,
            eventual_any_existential: counts_eventual_any as f64 / total_f,
            direct_any_existential: counts_direct_any as f64 / total_f,
            direct_none: counts_direct_none as f64 / total_f,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum RuleCategory {
    Structured,
    SemiStructured,
    LooselyStructured,
}

fn category_to_classification(category: RuleCategory) -> Classification {
    match category {
        RuleCategory::Structured => Classification::Structured,
        RuleCategory::SemiStructured => Classification::SemiStructured,
        RuleCategory::LooselyStructured => Classification::LooselyStructured,
    }
}

type RuleCheckResult = (bool, Vec<bool>);

fn check_rule_u1(p: &CalculatedPercentages) -> bool {
    // println!("Checking U1 rule: none_none > 0.80 ({}) && eventual_any_existential < 0.10 ({}) && direct_any_existential < 0.10 ({})",
    //     p.none_none, p.eventual_any_existential, p.direct_any_existential);
    (p.none_none > 0.80) && (p.eventual_any_existential < 0.10) && (p.direct_any_existential < 0.10)
}

fn check_rule_u2(p: &CalculatedPercentages) -> bool {
    // println!(
    //     "Checking U2 rule: none_equivalence > 0.80 ({})",
    //     p.none_equivalence
    // );
    p.none_equivalence > 0.80
}

fn check_rule_s1(p: &CalculatedPercentages) -> RuleCheckResult {
    let conds = vec![
        p.none_none < 0.05,
        p.none_implication < 0.10,
        p.eventual_equivalence > 0.10,
        p.eventual_implication > 0.40,
    ];
    // println!("Checking S1 rule: none_none < 0.05 ({}), none_implication < 0.10 ({}), eventual_equivalence > 0.10 ({}), eventual_implication > 0.40 ({})",
    //     p.none_none, p.none_implication, p.eventual_equivalence, p.eventual_implication);
    (conds.iter().all(|&c| c), conds)
}

fn check_rule_s2(p: &CalculatedPercentages) -> RuleCheckResult {
    let conds = vec![
        p.none_none < 0.05,
        p.none_implication <= 0.15,
        p.eventual_equivalence >= 0.10,
        p.eventual_implication > 0.30,
    ];
    // println!("Checking S2 rule: none_none < 0.05 ({}), none_implication <= 0.20 ({}), eventual_equivalence >= 0.10 ({}), eventual_implication > 0.30 ({})",
    //     p.none_none, p.none_implication, p.eventual_equivalence, p.eventual_implication);
    (conds.iter().all(|&c| c), conds)
}

fn check_rule_s3(p: &CalculatedPercentages) -> RuleCheckResult {
    let conds = vec![p.direct_none > 0.50];
    // println!("Checking S3 rule: direct_none > 0.50 ({})", p.direct_none);
    (conds.iter().all(|&c| c), conds)
}

fn check_rule_ss1(p: &CalculatedPercentages) -> RuleCheckResult {
    let conds = vec![
        p.none_none < 0.35,
        p.none_implication > 0.30,
        p.eventual_equivalence < 0.05,
        p.eventual_implication < 0.20,
    ];
    // println!("Checking SS1 rule: none_none < 0.35 ({}), none_implication > 0.30 ({}), eventual_equivalence < 0.05 ({}), eventual_implication < 0.20 ({})",
    //     p.none_none, p.none_implication, p.eventual_equivalence, p.eventual_implication);
    (conds.iter().all(|&c| c), conds)
}

fn check_rule_ss2(p: &CalculatedPercentages) -> RuleCheckResult {
    let conds = vec![
        p.none_none < 0.25,
        p.none_implication > 0.01,
        p.eventual_equivalence > 0.10,
        p.eventual_implication < 0.40,
    ];
    // println!("Checking SS2 rule: none_none < 0.25 ({}), none_implication > 0.01 ({}), eventual_equivalence > 0.10 ({}), eventual_implication < 0.40 ({})",
    //     p.none_none, p.none_implication, p.eventual_equivalence, p.eventual_implication);
    (conds.iter().all(|&c| c), conds)
}

fn check_rule_ls1(p: &CalculatedPercentages) -> RuleCheckResult {
    let conds = vec![
        p.none_none > 0.20,
        p.none_implication < 0.35,
        p.eventual_equivalence < 0.10,
        p.eventual_implication < 0.30,
    ];
    // println!("Checking LS1 rule: none_none > 0.20 ({}), none_implication < 0.35 ({}), eventual_equivalence < 0.10 ({}), eventual_implication < 0.30 ({})",
    //     p.none_none, p.none_implication, p.eventual_equivalence, p.eventual_implication);
    (conds.iter().all(|&c| c), conds)
}

fn check_rule_ls2(p: &CalculatedPercentages) -> RuleCheckResult {
    let conds = vec![
        p.none_none > 0.50,
        p.none_implication < 0.10,
        p.eventual_equivalence < 0.05,
        p.eventual_implication < 0.25,
    ];
    // println!("Checking LS2 rule: none_none > 0.50 ({}), none_implication < 0.10 ({}), eventual_equivalence < 0.05 ({}), eventual_implication < 0.25 ({})",
    //     p.none_none, p.none_implication, p.eventual_equivalence, p.eventual_implication);
    (conds.iter().all(|&c| c), conds)
}

fn apply_primary_rules(p: &CalculatedPercentages) -> (HashSet<RuleCategory>, Vec<RuleCheckResult>) {
    // println!("Applying primary rules...");
    let mut matched_categories = HashSet::new();
    let mut rule_results = Vec::new();

    let s1_res = check_rule_s1(p);
    if s1_res.0 {
        // println!("S1 rule matched!");
        matched_categories.insert(RuleCategory::Structured);
    }
    rule_results.push(s1_res);

    let s2_res = check_rule_s2(p);
    if s2_res.0 {
        // println!("S2 rule matched!");
        matched_categories.insert(RuleCategory::Structured);
    }
    rule_results.push(s2_res);

    let s3_res = check_rule_s3(p);
    if s3_res.0 {
        // println!("S3 rule matched!");
        matched_categories.insert(RuleCategory::Structured);
    }
    rule_results.push(s3_res);

    let ss1_res = check_rule_ss1(p);
    if ss1_res.0 {
        // println!("SS1 rule matched!");
        matched_categories.insert(RuleCategory::SemiStructured);
    }
    rule_results.push(ss1_res);

    let ss2_res = check_rule_ss2(p);
    if ss2_res.0 {
        // println!("SS2 rule matched!");
        matched_categories.insert(RuleCategory::SemiStructured);
    }
    rule_results.push(ss2_res);

    let ls1_res = check_rule_ls1(p);
    if ls1_res.0 {
        // println!("LS1 rule matched!");
        matched_categories.insert(RuleCategory::LooselyStructured);
    }
    rule_results.push(ls1_res);

    let ls2_res = check_rule_ls2(p);
    if ls2_res.0 {
        // println!("LS2 rule matched!");
        matched_categories.insert(RuleCategory::LooselyStructured);
    }
    rule_results.push(ls2_res);

    // println!("Primary rules matched categories: {:?}", matched_categories);
    (matched_categories, rule_results)
}

fn check_rule_bs1(p: &CalculatedPercentages) -> RuleCheckResult {
    let conds = vec![
        p.none_none < 0.10,
        p.none_negated_equivalence > 0.50, // This implies event_implication and eventual_equivalence are low.
        p.eventual_implication > 0.60, // This might conflict with none_negated_equivalence > 0.50 if they share matrix entries
    ];
    // println!("Checking BS1 rule: none_none < 0.10 ({}), none_negated_equivalence > 0.50 ({}), eventual_implication > 0.60 ({})",
    // p.none_none, p.none_negated_equivalence, p.eventual_implication);
    (conds.iter().all(|&c| c), conds)
}

fn check_rule_bs2(p: &CalculatedPercentages) -> RuleCheckResult {
    let conds = vec![p.none_none < 0.20, p.none_implication > 0.40];
    // println!(
    //     "Checking BS2 rule: none_none < 0.20 ({}), none_implication > 0.40 ({})",
    //     p.none_none, p.none_implication
    // );
    (conds.iter().all(|&c| c), conds)
}

fn check_rule_bl1(p: &CalculatedPercentages) -> RuleCheckResult {
    let conds = vec![p.none_none > 0.60, p.none_implication < 0.30];
    // println!(
    //     "Checking BL1 rule: none_none > 0.60 ({}), none_implication < 0.30 ({})",
    //     p.none_none, p.none_implication
    // );
    (conds.iter().all(|&c| c), conds)
}

fn apply_secondary_rules(
    p: &CalculatedPercentages,
) -> (HashSet<RuleCategory>, Vec<RuleCheckResult>) {
    // println!("Applying secondary rules...");
    let mut matched_categories = HashSet::new();
    let mut rule_results = Vec::new();

    let bs1_res = check_rule_bs1(p);
    if bs1_res.0 {
        // println!("BS1 rule matched!");
        matched_categories.insert(RuleCategory::Structured);
    }
    rule_results.push(bs1_res);

    let bs2_res = check_rule_bs2(p);
    if bs2_res.0 {
        // println!("BS2 rule matched!");
        matched_categories.insert(RuleCategory::SemiStructured);
    }
    rule_results.push(bs2_res);

    let bl1_res = check_rule_bl1(p);
    if bl1_res.0 {
        // println!("BL1 rule matched!");
        matched_categories.insert(RuleCategory::LooselyStructured);
    }
    rule_results.push(bl1_res);

    // println!(
    //     "Secondary rules matched categories: {:?}",
    //     matched_categories
    // );
    (matched_categories, rule_results)
}

fn calculate_by_most_indicators(
    primary_rule_check_results: &[RuleCheckResult],
    secondary_rule_check_results: &[RuleCheckResult],
) -> Classification {
    // println!("Calculating by most indicators...");
    let count_true_conditions = |bools: &[bool]| bools.iter().filter(|&&b| b).count();

    let s1_indicators = count_true_conditions(&primary_rule_check_results[0].1);
    let s2_indicators = count_true_conditions(&primary_rule_check_results[1].1);
    let s3_indicators = count_true_conditions(&primary_rule_check_results[2].1);
    let bs1_indicators = count_true_conditions(&secondary_rule_check_results[0].1);
    let score_structured = (s1_indicators + s2_indicators + s3_indicators) * 2 + bs1_indicators;

    let ss1_indicators = count_true_conditions(&primary_rule_check_results[3].1);
    let ss2_indicators = count_true_conditions(&primary_rule_check_results[4].1);
    let bs2_indicators = count_true_conditions(&secondary_rule_check_results[1].1);
    let score_semi_structured = (ss1_indicators + ss2_indicators) * 2 + bs2_indicators;

    let ls1_indicators = count_true_conditions(&primary_rule_check_results[5].1);
    let ls2_indicators = count_true_conditions(&primary_rule_check_results[6].1);
    let bl1_indicators = count_true_conditions(&secondary_rule_check_results[2].1);
    let score_loosely_structured = (ls1_indicators + ls2_indicators) * 2 + bl1_indicators;

    let scores = [
        (score_structured, RuleCategory::Structured),
        (score_semi_structured, RuleCategory::SemiStructured),
        (score_loosely_structured, RuleCategory::LooselyStructured),
    ];

    // println!(
    //     "Indicator scores: Structured={}, SemiStructured={}, LooselyStructured={}",
    //     score_structured, score_semi_structured, score_loosely_structured
    // );

    let max_score = scores.iter().map(|(s, _)| s).max().copied().unwrap_or(0);

    if max_score == 0 {
        // println!("No category had significant indicators.");
        return Classification::Error("No category had significant indicators.".to_string());
    }

    let top_categories: Vec<RuleCategory> = scores
        .iter()
        .filter(|(s, _)| *s == max_score)
        .map(|(_, c)| *c)
        .collect();

    // println!("Top categories: {:?}", top_categories);

    match top_categories.len() {
        1 => {
            let result = category_to_classification(top_categories[0]);
            // println!("Single top category: {}", result);
            result
        }
        2 => {
            let has_s = top_categories.contains(&RuleCategory::Structured);
            let has_ss = top_categories.contains(&RuleCategory::SemiStructured);
            let has_ls = top_categories.contains(&RuleCategory::LooselyStructured);

            let result = if has_s && has_ss {
                Classification::StructuredSemiStructured
            } else if has_ss && has_ls {
                Classification::SemiStructuredLooselyStructured
            } else if has_s && has_ls {
                Classification::SemiStructured
            } else {
                Classification::Error("Unexpected combination in top categories (2).".to_string())
            };
            // println!("Two top categories: {}", result);
            result
        }
        3 => {
            // println!("All three categories tied");
            Classification::SemiStructured
        }
        _ => {
            // println!("No category had a top score in most indicators");
            Classification::Error(
                "No category had a top score in most indicators (or internal error).".to_string(),
            )
        }
    }
}

pub fn classify_matrix(matrix: &InputMatrix) -> Classification {
    // println!("Starting classification...");
    let percentages = match CalculatedPercentages::new(matrix) {
        Ok(p) => {
            // println!("Calculated percentages: {:?}", p);
            p
        }
        Err(e) => {
            // println!("Error calculating percentages: {}", e);
            return Classification::Error(e);
        }
    };

    // println!("Checking unstructured rules...");
    if check_rule_u1(&percentages) {
        // println!("U1 rule matched - returning Unstructured");
        return Classification::Unstructured;
    }

    if check_rule_u2(&percentages) {
        // println!("U2 rule matched - returning Unstructured");
        return Classification::Unstructured;
    }

    // println!("Applying primary rules...");
    let (primary_matched_categories_set, primary_rule_results_for_indicators) =
        apply_primary_rules(&percentages);

    match primary_matched_categories_set.len() {
        0 => {
            // println!("No primary rules matched");
        }
        1 => {
            let category = primary_matched_categories_set.iter().next().unwrap();
            // println!("Single primary rule matched: {:?}", category);
            return category_to_classification(*category);
        }
        _ => {
            // println!(
            //     "Multiple primary rules matched: {:?}",
            //     primary_matched_categories_set
            // );
            let s_matched = primary_matched_categories_set.contains(&RuleCategory::Structured);
            let ss_matched = primary_matched_categories_set.contains(&RuleCategory::SemiStructured);
            let ls_matched =
                primary_matched_categories_set.contains(&RuleCategory::LooselyStructured);

            if s_matched && ss_matched && !ls_matched {
                // println!("Structured and SemiStructured matched");
                return Classification::StructuredSemiStructured;
            } else if !s_matched && ss_matched && ls_matched {
                // println!("SemiStructured and LooselyStructured matched");
                return Classification::SemiStructuredLooselyStructured;
            } else if s_matched && !ss_matched && ls_matched {
                // println!("Structured and LooselyStructured matched");
            } else if s_matched && ss_matched && ls_matched {
                // println!("All three primary categories matched");
            }
        }
    }

    // println!("Applying secondary rules...");
    let (secondary_matched_categories_set, secondary_rule_results_for_indicators) =
        apply_secondary_rules(&percentages);

    match secondary_matched_categories_set.len() {
        0 => {
            // println!("No secondary rules matched");
        }
        1 => {
            if primary_matched_categories_set.is_empty() || primary_matched_categories_set.len() > 1
            {
                let category = secondary_matched_categories_set.iter().next().unwrap();
                // println!("Single secondary rule matched: {:?}", category);
                return category_to_classification(*category);
            }
        }
        _ => {
            // println!("Multiple secondary rules matched");
        }
    }

    // println!("Falling back to most indicators calculation");
    calculate_by_most_indicators(
        &primary_rule_results_for_indicators,
        &secondary_rule_results_for_indicators,
    )
}

// ... [rest of the code remains the same] ...

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dependency_types::{
        dependency::Dependency as DetailedDependency, // Renamed to avoid clash
        existential::{
            DependencyType as ExistentialEnum, Direction as ExistentialDirection,
            ExistentialDependency,
        },
        temporal::{
            DependencyType as TemporalEnum, Direction as TemporalDirection, TemporalDependency,
        },
    };

    // Helper to create DetailedDependency
    fn dd(
        from: &str,
        to: &str,
        temporal: Option<(TemporalEnum, TemporalDirection)>,
        existential: Option<(ExistentialEnum, ExistentialDirection)>,
    ) -> DetailedDependency {
        DetailedDependency::new(
            from.to_string(),
            to.to_string(),
            temporal.map(|(t_type, t_dir)| TemporalDependency::new(from, to, t_type, t_dir)),
            existential.map(|(e_type, e_dir)| ExistentialDependency::new(from, to, e_type, e_dir)),
        )
    }

    // Simpler aliases for enums used in tests
    fn t_dir() -> TemporalEnum {
        TemporalEnum::Direct
    }
    fn t_ev() -> TemporalEnum {
        TemporalEnum::Eventual
    }
    fn t_fwd() -> TemporalDirection {
        TemporalDirection::Forward
    }
    #[allow(dead_code)]
    fn t_bwd() -> TemporalDirection {
        TemporalDirection::Backward
    }

    fn e_imp() -> ExistentialEnum {
        ExistentialEnum::Implication
    }
    fn e_eq() -> ExistentialEnum {
        ExistentialEnum::Equivalence
    }
    fn e_neq() -> ExistentialEnum {
        ExistentialEnum::NegatedEquivalence
    }
    fn e_fwd() -> ExistentialDirection {
        ExistentialDirection::Forward
    }
    #[allow(dead_code)]
    fn e_bwd() -> ExistentialDirection {
        ExistentialDirection::Backward
    }
    fn e_both() -> ExistentialDirection {
        ExistentialDirection::Both
    }

    // Helper function to build a matrix from counts for a total of 100 entries
    // Order of counts in the array:
    // 0: (None, None) -> nn
    // 1: (None, Implication) -> ni (assume Implication FWD for simplicity in test setup)
    // 2: (None, Equivalence) -> neq (assume Equivalence BOTH)
    // 3: (None, NegatedEquivalence) -> nneq (assume NEq BOTH)
    // 4: (Direct FWD, None) -> dn
    // 5: (Direct FWD, Implication FWD) -> di
    // 6: (Direct FWD, Equivalence BOTH) -> deq
    // 7: (Eventual FWD, None) -> en
    // 8: (Eventual FWD, Implication FWD) -> ei
    // 9: (Eventual FWD, Equivalence BOTH) -> eeq
    fn build_detailed_matrix_from_counts_array(counts: [usize; 10]) -> InputMatrix {
        let mut matrix = InputMatrix::new();
        let mut counter = 0;

        let mut add_entries = |count: usize, detailed_dep_template: DetailedDependency| {
            for _ in 0..count {
                // Create unique keys for each entry
                let from_act = format!("A{}", counter);
                let to_act = format!("B{}", counter);
                // Clone template and update from/to for this specific entry
                let mut dep_instance = detailed_dep_template.clone();
                dep_instance.from = from_act.clone();
                dep_instance.to = to_act.clone();
                if let Some(td) = &mut dep_instance.temporal_dependency {
                    td.from = from_act.clone();
                    td.to = to_act.clone();
                }
                if let Some(ed) = &mut dep_instance.existential_dependency {
                    ed.from = from_act.clone();
                    ed.to = to_act.clone();
                }
                matrix.insert((from_act, to_act), dep_instance);
                counter += 1;
            }
        };

        // Define templates for each dependency type used in counts array
        // NOTE: The actual 'from' and 'to' strings in the template don't matter here,
        // as they will be overridden by add_entries.
        add_entries(counts[0], dd("from", "to", None, None)); // (None, None)
        add_entries(counts[1], dd("from", "to", None, Some((e_imp(), e_fwd())))); // (None, Implication FWD)
        add_entries(counts[2], dd("from", "to", None, Some((e_eq(), e_both())))); // (None, Equivalence BOTH)
        add_entries(counts[3], dd("from", "to", None, Some((e_neq(), e_both())))); // (None, NegatedEquivalence BOTH)
        add_entries(counts[4], dd("from", "to", Some((t_dir(), t_fwd())), None)); // (Direct FWD, None)
        add_entries(
            counts[5],
            dd(
                "from",
                "to",
                Some((t_dir(), t_fwd())),
                Some((e_imp(), e_fwd())),
            ),
        ); // (Direct FWD, Implication FWD)
        add_entries(
            counts[6],
            dd(
                "from",
                "to",
                Some((t_dir(), t_fwd())),
                Some((e_eq(), e_both())),
            ),
        ); // (Direct FWD, Equivalence BOTH)
        add_entries(counts[7], dd("from", "to", Some((t_ev(), t_fwd())), None)); // (Eventual FWD, None)
        add_entries(
            counts[8],
            dd(
                "from",
                "to",
                Some((t_ev(), t_fwd())),
                Some((e_imp(), e_fwd())),
            ),
        ); // (Eventual FWD, Implication FWD)
        add_entries(
            counts[9],
            dd(
                "from",
                "to",
                Some((t_ev(), t_fwd())),
                Some((e_eq(), e_both())),
            ),
        ); // (Eventual FWD, Equivalence BOTH)

        let total_provided_counts: usize = counts.iter().sum();
        assert_eq!(
            matrix.len(),
            total_provided_counts,
            "Matrix length does not match sum of provided counts."
        );

        if total_provided_counts != 100 && total_provided_counts != 0 {
            // Allow 0 for empty test
            eprintln!(
                "Warning: Test counts sum to {} not 100. Percentages might be skewed if not intended.",
                total_provided_counts
            );
        }
        matrix
    }

    #[test]
    fn test_empty_matrix() {
        let matrix = InputMatrix::new();
        assert_eq!(
            classify_matrix(&matrix),
            Classification::Error("Input matrix is empty".to_string())
        );
    }

    #[test]
    fn test_unstructured_u1_exact() {
        // U1: (None, None) > 80% && (Eventual, Any) < 10% && (Direct, Any) < 10%
        // Counts: [NN, NI, NEq, NNEq, DN, DI, DEq, EN, EI, EEq]
        let counts = [81, 0, 0, 0, 5, 0, 0, 5, 0, 0]; // NN=81%, DN=5%, EN=5%. EventualAny=5%, DirectAny=5%
        let matrix = build_detailed_matrix_from_counts_array(counts);
        assert_eq!(classify_matrix(&matrix), Classification::Unstructured);
    }

    #[test]
    fn test_unstructured_u2_exact() {
        // U2: (None, Equivalence) > 80%
        let counts = [0, 0, 81, 0, 0, 0, 0, 0, 0, 19]; // NEq = 81%, fill with EEq
        let matrix = build_detailed_matrix_from_counts_array(counts);
        assert_eq!(classify_matrix(&matrix), Classification::Unstructured);
    }

    #[test]
    fn test_primary_s1_and_s2_match_structured() {
        // S1/S2 like conditions:
        // - None,None < 5% (e.g. 4%)
        // - None,Implication < 10% (e.g. 9%)
        // - Eventual,Equivalence > 10% (e.g. 11%)
        // - Eventual,Implication > 40% (e.g. 41%)
        // Remainder: 100 - 4 - 9 - 11 - 41 = 35
        let counts = [4, 9, 0, 0, 35, 0, 0, 0, 41, 11]; // NN=4%, NI=9%, EEq=11%, EI=41%, Fill with DN=35%
        let matrix = build_detailed_matrix_from_counts_array(counts);
        assert_eq!(classify_matrix(&matrix), Classification::Structured);
    }

    #[test]
    fn test_secondary_bs2_leads_to_semistructured() {
        // BS2 rule: None,None < 20% && None,Implication > 40%
        // Counts: NN=19%, NI=41% (total 60%). Remainder 40%. Let's put into EEq.
        let counts = [19, 41, 0, 0, 0, 0, 0, 0, 0, 40];
        let matrix = build_detailed_matrix_from_counts_array(counts);
        // This test requires primary rules to not match definitively.
        // S1/S2: NN < 5% (fails, NN=19%).
        // SS1: NN < 35% (ok), NI > 30% (ok), EEq < 5% (ok, EEq=0 if we put remainder elsewhere or EEq=40 if here), EI < 20% (ok).
        //   If EEq=40%, SS1 fails. If EEq=0, then SS1 might match.
        //   Let's assume it falls through to secondary.
        assert_eq!(classify_matrix(&matrix), Classification::SemiStructured);
    }

    // Synthetic logs tests
    #[test]
    fn test_log01_structured() {
        let counts = [0, 0, 7, 13, 0, 13, 7, 0, 47, 13]; // nn,ni,neq,nneq, dn,di,deq, en,ei,eeq
        let matrix = build_detailed_matrix_from_counts_array(counts);
        assert_eq!(classify_matrix(&matrix), Classification::Structured);
    }

    #[test]
    fn test_log04_structured() {
        let counts = [0, 0, 7, 7, 0, 13, 0, 0, 40, 33];
        let matrix = build_detailed_matrix_from_counts_array(counts);
        assert_eq!(classify_matrix(&matrix), Classification::Structured);
    }

    #[test]
    fn test_log05_structured() {
        let counts = [0, 0, 0, 27, 53, 0, 0, 7, 13, 0];
        let matrix = build_detailed_matrix_from_counts_array(counts);
        assert_eq!(classify_matrix(&matrix), Classification::Structured);
    }

    #[test]
    fn test_log09_unstructured() {
        let counts = [0, 0, 100, 0, 0, 0, 0, 0, 0, 0];
        let matrix = build_detailed_matrix_from_counts_array(counts);
        assert_eq!(classify_matrix(&matrix), Classification::Unstructured);
    }

    #[test]
    fn test_log06_semistructured() {
        let counts = [0, 28, 5, 0, 0, 0, 10, 0, 0, 57];
        let matrix = build_detailed_matrix_from_counts_array(counts);
        assert_eq!(classify_matrix(&matrix), Classification::SemiStructured);
    }

    #[test]
    fn test_log02_semistructured() {
        let counts = [13, 47, 13, 7, 0, 13, 7, 0, 0, 0];
        let matrix = build_detailed_matrix_from_counts_array(counts);
        assert_eq!(classify_matrix(&matrix), Classification::SemiStructured);
    }

    #[test]
    fn test_log07_semistructured() {
        let counts = [6, 21, 11, 3, 0, 11, 6, 0, 17, 25];
        let matrix = build_detailed_matrix_from_counts_array(counts);
        assert_eq!(classify_matrix(&matrix), Classification::SemiStructured);
    }

    #[test]
    fn test_log10_semistructured() {
        let counts = [5, 19, 5, 0, 0, 0, 5, 0, 28, 38];
        let matrix = build_detailed_matrix_from_counts_array(counts);
        assert_eq!(classify_matrix(&matrix), Classification::SemiStructured);
    }

    #[test]
    fn test_log03_looselystructured() {
        let counts = [60, 7, 7, 13, 0, 0, 0, 0, 13, 0];
        let matrix = build_detailed_matrix_from_counts_array(counts);
        assert_eq!(classify_matrix(&matrix), Classification::LooselyStructured);
    }

    #[test]
    fn test_log08_looselystructured() {
        let counts = [23, 14, 0, 14, 0, 10, 0, 10, 24, 5];
        let matrix = build_detailed_matrix_from_counts_array(counts);
        assert_eq!(classify_matrix(&matrix), Classification::LooselyStructured);
    }

    #[test]
    fn test_log11_looselystructured() {
        let counts = [66, 7, 7, 0, 0, 0, 0, 0, 20, 0];
        let matrix = build_detailed_matrix_from_counts_array(counts);
        assert_eq!(classify_matrix(&matrix), Classification::LooselyStructured);
    }

    #[test]
    fn test_log12_structured() {
        let counts = [0, 0, 6, 35, 3, 14, 0, 6, 25, 11];
        let matrix = build_detailed_matrix_from_counts_array(counts);
        assert_eq!(classify_matrix(&matrix), Classification::Structured);
    }

    #[test]
    fn test_log13_semistructured() {
        let counts = [22, 2, 2, 16, 0, 0, 0, 15, 30, 13];
        let matrix = build_detailed_matrix_from_counts_array(counts);
        assert_eq!(classify_matrix(&matrix), Classification::SemiStructured);
    }

    #[test]
    fn test_log14_semistructured_looselystructured() {
        let counts = [33, 33, 0, 17, 0, 0, 0, 0, 17, 0];
        let matrix = build_detailed_matrix_from_counts_array(counts);
        assert_eq!(
            classify_matrix(&matrix),
            Classification::SemiStructuredLooselyStructured
        );
    }

    #[test]
    fn test_log15_structured() {
        let counts = [0, 0, 8, 8, 0, 11, 3, 11, 44, 15];
        let matrix = build_detailed_matrix_from_counts_array(counts);
        assert_eq!(classify_matrix(&matrix), Classification::Structured);
    }

    #[test]
    fn test_log16_looselystructured() {
        let counts = [80, 0, 10, 0, 0, 0, 0, 10, 0, 0]; // Counts: NN, NI, NEq, NNEq, DN, DI, DEq, EN, EI, EEq
        let matrix = build_detailed_matrix_from_counts_array(counts);
        assert_eq!(classify_matrix(&matrix), Classification::LooselyStructured);
    }

    #[test]
    fn test_log17_semistructured() {
        let counts = [14, 33, 3, 0, 0, 0, 3, 0, 22, 25];
        let matrix = build_detailed_matrix_from_counts_array(counts);
        assert_eq!(classify_matrix(&matrix), Classification::SemiStructured);
    }

    #[test]
    fn test_log18_structured() {
        let counts = [0, 20, 20, 0, 0, 0, 0, 10, 40, 10];
        let matrix = build_detailed_matrix_from_counts_array(counts);
        assert_eq!(classify_matrix(&matrix), Classification::Structured);
    }

    #[test]
    fn test_log19_structured() {
        let counts = [0, 20, 20, 10, 0, 0, 0, 0, 40, 10];
        let matrix = build_detailed_matrix_from_counts_array(counts);
        assert_eq!(classify_matrix(&matrix), Classification::Structured);
    }
}
