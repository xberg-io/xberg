use super::{Error, Limits, Range, Work, projection};
use crate::extractors::security::SecurityLimits;
use crate::text::redaction::patterns::{BorrowedMatch, visit_category};
use crate::types::redaction::{PiiCategory, RedactionFinding, RedactionStrategy};
use std::{cell::RefCell, collections::HashSet, mem::size_of};

pub(in crate::text::redaction) struct Matchers<'a> {
    pub categories: &'a [PiiCategory],
    pub requested: &'a HashSet<PiiCategory>,
    pub custom: &'a [(String, regex::Regex)],
    pub ner: &'a [(PiiCategory, regex::Regex)],
    pub external: &'a [(PiiCategory, regex::Regex)],
    pub builtins: bool,
}

#[derive(Clone, Copy)]
enum Category<'a> {
    Known(&'a PiiCategory),
    Custom(&'a str),
}
impl<'a> Category<'a> {
    fn custom(self) -> Option<&'a str> {
        match self {
            Self::Custom(label) => Some(label),
            Self::Known(PiiCategory::Custom(label)) => Some(label.as_str()),
            _ => None,
        }
    }
    fn label_bytes(self) -> usize {
        match self {
            Self::Custom(label) => label.len(),
            Self::Known(PiiCategory::Custom(label)) => label.len(),
            Self::Known(
                PiiCategory::Email
                | PiiCategory::Phone
                | PiiCategory::Ssn
                | PiiCategory::CreditCard
                | PiiCategory::PostalCode
                | PiiCategory::IpAddress
                | PiiCategory::Iban
                | PiiCategory::SwiftBic
                | PiiCategory::DateOfBirth
                | PiiCategory::Person
                | PiiCategory::Organization
                | PiiCategory::Location,
            ) => "date_of_birth".len(),
        }
    }
    fn same(self, other: Self) -> bool {
        match (self, other) {
            (Self::Known(a), Self::Known(b)) => a == b,
            _ => self.custom().is_some() && self.custom() == other.custom(),
        }
    }
    fn owned(self) -> PiiCategory {
        match self {
            Self::Known(category) => category.clone(),
            Self::Custom(label) => PiiCategory::Custom(label.to_owned()),
        }
    }
}

struct Candidate<'a> {
    raw: Range<usize>,
    identity: Range<usize>,
    decoded: bool,
    category: Category<'a>,
}

pub(in crate::text::redaction) struct LiteralMatch {
    pub raw: Range<usize>,
    pub identity: Range<usize>,
    pub decoded: bool,
    pub category: PiiCategory,
}

pub(in crate::text::redaction) struct LiteralPlan {
    pub source: String,
    pub decoded: String,
    pub matches: Vec<LiteralMatch>,
    pub replacements: Vec<(usize, usize, String)>,
}

fn streams(category: &PiiCategory) -> usize {
    match category {
        PiiCategory::IpAddress => 2,
        PiiCategory::PostalCode => 3,
        PiiCategory::Email
        | PiiCategory::Phone
        | PiiCategory::Ssn
        | PiiCategory::CreditCard
        | PiiCategory::Iban
        | PiiCategory::SwiftBic => 1,
        _ => 0,
    }
}

impl<'a> Matchers<'a> {
    fn scan<'s>(
        &self,
        text: &'s str,
        limits: Limits,
        work: &RefCell<Work>,
        eligible: &mut impl FnMut(BorrowedMatch<'s>, Category<'a>) -> Result<(), Error>,
    ) -> Result<(), Error> {
        if self.builtins {
            for category in self.categories {
                let count = streams(category);
                work.borrow_mut()
                    .charge(text.len().checked_mul(count).ok_or(Error::Budget)?, limits)?;
                if matches!(category, PiiCategory::Iban | PiiCategory::PostalCode) {
                    work.borrow_mut().reserve(text.len(), limits)?;
                }
                visit_category(
                    text,
                    category,
                    &mut |hit| {
                        let mut budget = work.borrow_mut();
                        if matches!(category, PiiCategory::CreditCard | PiiCategory::Iban) {
                            budget.reserve(hit.text.len(), limits)?;
                        }
                        budget.charge(hit.text.len().checked_add(1).ok_or(Error::Budget)?, limits)
                    },
                    &mut |hit| eligible(hit, Category::Known(category)),
                )?;
            }
        }
        let custom = self.custom.iter().map(|(label, re)| (Category::Custom(label), re));
        let other = self
            .ner
            .iter()
            .chain(self.external)
            .map(|(category, re)| (Category::Known(category), re));
        for (category, regex) in custom.chain(other) {
            work.borrow_mut().charge(text.len(), limits)?;
            for hit in regex.find_iter(text) {
                work.borrow_mut().charge(1, limits)?;
                if !hit.is_empty()
                    && (category.custom().is_some()
                        || self.requested.is_empty()
                        || matches!(category, Category::Known(value) if self.requested.contains(value)))
                {
                    eligible(
                        BorrowedMatch {
                            start: hit.start(),
                            end: hit.end(),
                            text: hit.as_str(),
                        },
                        category,
                    )?;
                }
            }
        }
        Ok(())
    }
}

fn push<'a>(
    records: &mut Vec<Candidate<'a>>,
    capacity: &mut usize,
    candidate: Candidate<'a>,
    bound: usize,
    limits: Limits,
    work: &mut Work,
) -> Result<(), Error> {
    work.charge(1, limits)?;
    if records.len() >= bound {
        return Err(Error::Budget);
    }
    if records.len() == *capacity {
        let next = capacity
            .checked_mul(2)
            .and_then(|n| n.checked_add(1))
            .ok_or(Error::Budget)?
            .min(bound);
        work.reserve(
            next.checked_mul(size_of::<Candidate<'_>>()).ok_or(Error::Budget)?,
            limits,
        )?;
        records
            .try_reserve_exact(next - records.len())
            .map_err(|_| Error::Budget)?;
        *capacity = next;
    }
    records.push(candidate);
    Ok(())
}

fn indices(count: usize) -> Result<Vec<usize>, Error> {
    let mut indices = Vec::new();
    indices.try_reserve_exact(count).map_err(|_| Error::Budget)?;
    indices.extend(0..count);
    Ok(indices)
}

fn ordered(records: &[Candidate<'_>], limits: Limits, work: &mut Work) -> Result<Vec<usize>, Error> {
    work.reserve(
        records.len().checked_mul(size_of::<usize>() * 3).ok_or(Error::Budget)?,
        limits,
    )?;
    work.charge(records.len().checked_mul(2).ok_or(Error::Budget)?, limits)?;
    let mut order = indices(records.len())?;
    let mut scratch = indices(records.len())?;
    let mut width = 1usize;
    while width < records.len() {
        let step = width.checked_mul(2).ok_or(Error::Budget)?;
        for start in (0..records.len()).step_by(step) {
            let mid = start.saturating_add(width).min(records.len());
            let end = start.saturating_add(step).min(records.len());
            let (mut left, mut right) = (start, mid);
            for target in &mut scratch[start..end] {
                work.charge(1, limits)?;
                let take_left = if left == mid {
                    false
                } else if right == end {
                    true
                } else {
                    work.charge(1, limits)?;
                    let a = &records[order[left]].raw;
                    let b = &records[order[right]].raw;
                    (a.start, std::cmp::Reverse(a.len()), order[left])
                        <= (b.start, std::cmp::Reverse(b.len()), order[right])
                };
                if take_left {
                    *target = order[left];
                    left += 1;
                } else {
                    *target = order[right];
                    right += 1;
                }
            }
        }
        std::mem::swap(&mut order, &mut scratch);
        width = step;
    }
    Ok(order)
}

fn prefer_semantic(
    records: &[Candidate<'_>],
    winner: usize,
    next: usize,
    limits: Limits,
    work: &mut Work,
) -> Result<bool, Error> {
    let (previous, candidate) = (&records[winner], &records[next]);
    if !candidate.decoded {
        return Ok(false);
    }
    work.charge(
        candidate
            .category
            .label_bytes()
            .checked_add(previous.category.label_bytes())
            .ok_or(Error::Budget)?,
        limits,
    )?;
    if !candidate.category.same(previous.category) {
        return Ok(false);
    }
    work.charge(1, limits)?;
    Ok(!previous.decoded
        || (
            candidate.identity.start,
            std::cmp::Reverse(candidate.identity.len()),
            next,
        ) < (
            previous.identity.start,
            std::cmp::Reverse(previous.identity.len()),
            winner,
        ))
}

fn semantic_coverage(
    records: &[Candidate<'_>],
    group: &[usize],
    winner: usize,
    strategy: RedactionStrategy,
    limits: Limits,
    work: &mut Work,
) -> Result<(), Error> {
    if !matches!(strategy, RedactionStrategy::Hash | RedactionStrategy::TokenReplace) {
        return Ok(());
    }
    let identity = &records[winner];
    for &index in group {
        work.charge(1, limits)?;
        let required = &records[index];
        if required.decoded
            && (!identity.decoded
                || identity.identity.start > required.identity.start
                || identity.identity.end < required.identity.end)
        {
            return Err(Error::Coverage);
        }
    }
    Ok(())
}

fn selected(
    records: &[Candidate<'_>],
    order: &[usize],
    strategy: RedactionStrategy,
    limits: Limits,
    work: &mut Work,
) -> Result<Vec<usize>, Error> {
    let mut selected = Vec::new();
    selected.try_reserve_exact(order.len()).map_err(|_| Error::Budget)?;
    let mut at = 0;
    let mut end = 0;
    while at < order.len() {
        work.charge(1, limits)?;
        let group_start = at;
        let first = &records[order[at]];
        let mut winner = order[at];
        at += 1;
        while at < order.len() {
            work.charge(1, limits)?;
            let next = &records[order[at]];
            if next.raw != first.raw {
                break;
            }
            if prefer_semantic(records, winner, order[at], limits, work)? {
                winner = order[at];
            }
            at += 1;
        }
        if first.raw.start >= end {
            semantic_coverage(records, &order[group_start..at], winner, strategy, limits, work)?;
            end = first.raw.end;
            selected.push(winner);
        }
    }
    Ok(selected)
}

fn cover(
    records: &[Candidate<'_>],
    order: &[usize],
    selected: &[usize],
    limits: Limits,
    work: &mut Work,
) -> Result<(), Error> {
    let mut first = 0;
    for &index in order {
        work.charge(1, limits)?;
        let raw = &records[index].raw;
        while first < selected.len() && records[selected[first]].raw.end <= raw.start {
            work.charge(1, limits)?;
            first += 1;
        }
        let mut end = raw.start;
        for &item in &selected[first..] {
            work.charge(1, limits)?;
            let retained = &records[item].raw;
            if retained.start > end {
                break;
            }
            end = end.max(retained.end);
            if end >= raw.end {
                break;
            }
        }
        if end < raw.end {
            return Err(Error::Coverage);
        }
    }
    Ok(())
}

fn replacement_bound(strategy: RedactionStrategy, category: Category<'_>) -> Result<usize, Error> {
    let label = category.label_bytes();
    match strategy {
        RedactionStrategy::Mask => Ok("[REDACTED]".len()),
        RedactionStrategy::Hash => Ok("[HASH:0000000000000000]".len()),
        RedactionStrategy::Drop => Ok(0),
        RedactionStrategy::TokenReplace => label
            .checked_add("[_]".len() + u32::MAX.ilog10() as usize + 1)
            .ok_or(Error::Budget),
    }
}

fn collect<'a>(
    source: &str,
    projection: &projection::Projection<'_>,
    matchers: &Matchers<'a>,
    limits: Limits,
    work: &RefCell<Work>,
) -> Result<Vec<Candidate<'a>>, Error> {
    let stream_count = matchers
        .categories
        .iter()
        .filter(|_| matchers.builtins)
        .try_fold(0usize, |sum, category| sum.checked_add(streams(category)))
        .and_then(|n| n.checked_add(matchers.custom.len()))
        .and_then(|n| n.checked_add(matchers.ner.len()))
        .and_then(|n| n.checked_add(matchers.external.len()))
        .ok_or(Error::Budget)?;
    let bound = source
        .len()
        .checked_add(projection.leaf.text.len())
        .and_then(|n| n.checked_add(2))
        .and_then(|n| n.checked_mul(stream_count))
        .ok_or(Error::Budget)?;
    let mut records = Vec::new();
    let mut capacity = 0;
    for decoded in [false, true] {
        let text = if decoded { &projection.leaf.text } else { source };
        matchers.scan(text, limits, work, &mut |hit, category| {
            if hit.start >= hit.end {
                return Ok(());
            }
            let mut budget = work.borrow_mut();
            let identity = hit.start..hit.end;
            if text.get(identity.clone()) != Some(hit.text) {
                return Err(Error::InvalidSource);
            }
            let raw = if decoded {
                let mapped = projection.map(identity.clone(), hit.text, limits, &mut budget)?;
                if mapped.semantic != identity {
                    return Err(Error::InvalidSource);
                }
                mapped.raw
            } else {
                identity.clone()
            };
            if source.get(raw.clone()).is_none() || u32::try_from(raw.end).is_err() {
                return Err(Error::InvalidSource);
            }
            push(
                &mut records,
                &mut capacity,
                Candidate {
                    raw,
                    identity,
                    decoded,
                    category,
                },
                bound,
                limits,
                &mut budget,
            )
        })?;
    }
    Ok(records)
}

fn prepare<'a>(
    source: &str,
    matchers: &Matchers<'a>,
    security: &SecurityLimits,
    strategy: RedactionStrategy,
    work: &RefCell<Work>,
) -> Result<LiteralPlan, Error> {
    let limits = Limits {
        input: security.max_content_size,
        owned: security.max_content_size,
        operations: security.max_iterations,
    };
    let projection = projection::project(source, limits, security.max_nesting_depth, &mut work.borrow_mut())?;
    if projection.leaf.source != (0..source.len()) {
        return Err(Error::InvalidSource);
    }
    let records = collect(source, &projection, matchers, limits, work)?;
    let mut budget = work.borrow_mut();
    let order = ordered(&records, limits, &mut budget)?;
    let selected = selected(&records, &order, strategy, limits, &mut budget)?;
    cover(&records, &order, &selected, limits, &mut budget)?;
    let count = selected.len();
    u32::try_from(count).map_err(|_| Error::Budget)?;
    budget.reserve(
        count
            .checked_mul(
                size_of::<LiteralMatch>() + size_of::<(usize, usize, String)>() + size_of::<RedactionFinding>(),
            )
            .ok_or(Error::Budget)?,
        limits,
    )?;
    let mut output_bound = source.len();
    for &index in &selected {
        budget.charge(1, limits)?;
        let item = &records[index];
        let replacement = replacement_bound(strategy, item.category)?;
        output_bound = output_bound.checked_add(replacement).ok_or(Error::Budget)?;
        let label = item.category.label_bytes();
        // Strategy token keys/labels and the audit token are copied only after coverage succeeds. ~keep
        let strings = replacement
            .checked_mul(3)
            .and_then(|n| label.checked_mul(5).and_then(|v| n.checked_add(v)))
            .and_then(|n| n.checked_add(item.identity.len()))
            .and_then(|n| n.checked_add("0123456789abcdef".len() * 4))
            .ok_or(Error::Budget)?;
        budget.reserve(strings, limits)?;
        budget.charge(strings, limits)?;
    }
    budget.charge(
        output_bound
            .checked_mul(count)
            .and_then(|n| n.checked_add(source.len()))
            .ok_or(Error::Budget)?,
        limits,
    )?;
    budget.reserve(output_bound, limits)?;
    let mut output = String::new();
    output.try_reserve_exact(output_bound).map_err(|_| Error::Budget)?;
    output.push_str(source);
    let mut matches = Vec::new();
    let mut replacements = Vec::new();
    matches.try_reserve_exact(count).map_err(|_| Error::Budget)?;
    replacements.try_reserve_exact(count).map_err(|_| Error::Budget)?;
    for index in selected {
        budget.charge(1, limits)?;
        let item = &records[index];
        matches.push(LiteralMatch {
            raw: item.raw.clone(),
            identity: item.identity.clone(),
            decoded: item.decoded,
            category: item.category.owned(),
        });
    }
    Ok(LiteralPlan {
        source: output,
        decoded: projection.leaf.text,
        matches,
        replacements,
    })
}

pub(in crate::text::redaction) fn plan(
    source: &str,
    matchers: Matchers<'_>,
    limits: &SecurityLimits,
    strategy: RedactionStrategy,
) -> crate::Result<LiteralPlan> {
    let work = RefCell::new(Work {
        entity_limit: Some(limits.max_entity_length),
        ..Default::default()
    });
    prepare(source, &matchers, limits, strategy, &work).map_err(|error| {
        let reason = match error {
            Error::Coverage => "coverage is incomplete",
            Error::Budget => work.borrow().failure.unwrap_or("requested helper capacity limit"),
            Error::Unsupported | Error::UnsupportedAtom => "unsupported syntax",
            _ => "invalid source mapping",
        };
        crate::XbergError::validation(format!("formatted literal Markdown redaction: {reason}"))
    })
}
