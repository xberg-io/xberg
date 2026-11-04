use super::*;

fn mask(length: usize, limits: Limits, work: &mut Work) -> Result<Vec<bool>, Error> {
    work.reserve(length.checked_mul(size_of::<bool>()).ok_or(Error::Budget)?, limits)?;
    work.charge(length, limits)?;
    let mut mask = Vec::new();
    mask.try_reserve_exact(length).map_err(|_| Error::Budget)?;
    mask.resize(length, false);
    Ok(mask)
}

fn mark(mask: &mut [bool], range: Range<usize>, limits: Limits, work: &mut Work) -> Result<(), Error> {
    let selected = mask.get_mut(range).ok_or(Error::InvalidSource)?;
    work.charge(selected.len(), limits)?;
    selected.fill(true);
    Ok(())
}

fn emptied(
    projection: &Projection<'_>,
    construct: &Construct,
    removed: &[bool],
    limits: Limits,
    work: &mut Work,
) -> Result<bool, Error> {
    if construct.decoded.is_empty() {
        return Ok(false);
    }
    let origins = &projection.leaf.origins;
    let search_bound = (usize::BITS - origins.len().leading_zeros()) as usize;
    work.charge(
        search_bound
            .checked_add(1)
            .and_then(|n| n.checked_mul(2))
            .ok_or(Error::Budget)?,
        limits,
    )?;
    let first = origins.partition_point(|origin| origin.decoded.end <= construct.decoded.start);
    let end = origins.partition_point(|origin| origin.decoded.start < construct.decoded.end);
    let origins = origins
        .get(first..end)
        .filter(|origins| !origins.is_empty())
        .ok_or(Error::InvalidSource)?;
    let mut touched = false;
    for origin in origins {
        let text = projection
            .leaf
            .text
            .get(origin.decoded.clone())
            .ok_or(Error::InvalidSource)?;
        let bytes = removed.get(origin.source.clone()).ok_or(Error::InvalidSource)?;
        work.charge(
            bytes
                .len()
                .checked_add(text.len())
                .and_then(|n| n.checked_add(1))
                .ok_or(Error::Budget)?,
            limits,
        )?;
        let covered = bytes.iter().all(|removed| *removed);
        touched |= covered;
        if !covered && !text.chars().next().ok_or(Error::InvalidSource)?.is_whitespace() {
            return Ok(false);
        }
    }
    Ok(touched)
}

fn collect(
    source: &str,
    mask: &[bool],
    bound: usize,
    limits: Limits,
    work: &mut Work,
) -> Result<Vec<Range<usize>>, Error> {
    work.reserve(
        bound.checked_mul(size_of::<Range<usize>>()).ok_or(Error::Budget)?,
        limits,
    )?;
    work.charge(mask.len(), limits)?;
    let mut output = Vec::new();
    output.try_reserve_exact(bound).map_err(|_| Error::Budget)?;
    let mut start = None;
    for index in 0..=mask.len() {
        let removed = mask.get(index).copied().unwrap_or(false);
        if removed && start.is_none() {
            start = Some(index);
        } else if !removed && let Some(start) = start.take() {
            if output.len() == bound || source.get(start..index).is_none() {
                return Err(Error::InvalidSource);
            }
            output.push(start..index);
        }
    }
    Ok(output)
}

pub(super) fn ranges(
    projection: &Projection<'_>,
    matches: impl ExactSizeIterator<Item = Range<usize>>,
    limits: Limits,
    work: &mut Work,
) -> Result<Vec<Range<usize>>, Error> {
    let count = matches.len();
    if count == 0 {
        return Ok(Vec::new());
    }
    let mut selected = mask(projection.source.len(), limits, work)?;
    for range in matches {
        projection.source.get(range.clone()).ok_or(Error::InvalidSource)?;
        mark(&mut selected, range, limits, work)?;
    }
    let mut rewrite = mask(selected.len(), limits, work)?;
    work.charge(selected.len(), limits)?;
    rewrite.copy_from_slice(&selected);
    for construct in &projection.constructs {
        work.charge(1, limits)?;
        if emptied(projection, construct, &selected, limits, work)? {
            mark(&mut rewrite, construct.raw.start..construct.payload.start, limits, work)?;
            mark(&mut rewrite, construct.payload.end..construct.raw.end, limits, work)?;
        }
    }
    let bound = projection
        .constructs
        .len()
        .checked_mul(2)
        .and_then(|n| n.checked_add(count))
        .ok_or(Error::Budget)?;
    collect(projection.source, &rewrite, bound, limits, work)
}
