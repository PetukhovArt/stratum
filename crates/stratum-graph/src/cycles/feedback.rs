use std::cmp::Reverse;
use std::collections::BinaryHeap;

use super::Local;

/// Eades-Lin-Smyth greedy ordering. Returns the indices into `local.edges` of
/// the edges pointing backwards in that ordering, self-loops included.
/// Depends only on local indices, so it is the same for any build order.
pub(super) fn back_edges(local: &Local) -> Vec<usize> {
    let n = local.n;
    let mut out_deg = vec![0_i32; n];
    let mut in_deg = vec![0_i32; n];
    let mut preds: Vec<Vec<u32>> = vec![Vec::new(); n];
    for &(u, v) in &local.edges {
        if u != v {
            out_deg[u as usize] += 1;
            in_deg[v as usize] += 1;
            preds[v as usize].push(u);
        }
    }

    let mut removed = vec![false; n];
    let mut sinks: Vec<u32> = Vec::new();
    let mut sources: Vec<u32> = Vec::new();
    let mut heap: BinaryHeap<(i32, Reverse<u32>)> = BinaryHeap::new();
    for v in (0..n).rev() {
        let id = to_id(v);
        if out_deg[v] == 0 {
            sinks.push(id);
        }
        if in_deg[v] == 0 {
            sources.push(id);
        }
        heap.push((out_deg[v] - in_deg[v], Reverse(id)));
    }

    let mut front: Vec<u32> = Vec::with_capacity(n);
    let mut back: Vec<u32> = Vec::new();
    let mut left = n;
    while left > 0 {
        let (v, to_front) = if let Some(v) = sinks.pop() {
            (v, false)
        } else if let Some(v) = sources.pop() {
            (v, true)
        } else if let Some((delta, Reverse(v))) = heap.pop() {
            if out_deg[v as usize] - in_deg[v as usize] != delta {
                continue;
            }
            (v, true)
        } else {
            break;
        };
        let vi = v as usize;
        if removed[vi] {
            continue;
        }
        removed[vi] = true;
        left -= 1;
        if to_front {
            front.push(v);
        } else {
            back.push(v);
        }
        for &w in &local.tg[local.off[vi]..local.off[vi + 1]] {
            let wi = w as usize;
            if removed[wi] || wi == vi {
                continue;
            }
            in_deg[wi] -= 1;
            if in_deg[wi] == 0 {
                sources.push(w);
            }
            heap.push((out_deg[wi] - in_deg[wi], Reverse(w)));
        }
        for &u in &preds[vi] {
            let ui = u as usize;
            if removed[ui] || ui == vi {
                continue;
            }
            out_deg[ui] -= 1;
            if out_deg[ui] == 0 {
                sinks.push(u);
            }
            heap.push((out_deg[ui] - in_deg[ui], Reverse(u)));
        }
    }

    let mut position = vec![0_usize; n];
    for (at, &v) in front.iter().chain(back.iter().rev()).enumerate() {
        position[v as usize] = at;
    }
    local
        .edges
        .iter()
        .enumerate()
        .filter(|&(_, &(u, v))| position[u as usize] >= position[v as usize])
        .map(|(e, _)| e)
        .collect()
}

fn to_id(v: usize) -> u32 {
    u32::try_from(v).unwrap_or(u32::MAX)
}
