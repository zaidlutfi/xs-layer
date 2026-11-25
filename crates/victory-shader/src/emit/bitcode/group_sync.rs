//! Convergence proof for a paired Wave64 LDS candidate. This runs on
//! the emitted control flow, including the loop caps and predicated loads.

use super::{Bin, Const, Module, Op, Term, TypeDef, V, pred};
use std::collections::HashSet;

mod paired_memory;

struct ControlFlow {
    succ: Vec<Vec<usize>>,
    pred: Vec<Vec<usize>>,
    reachable: Vec<bool>,
    dom: Dominators,
    postdom: Dominators,
}

impl ControlFlow {
    fn new(m: &Module) -> Option<Self> {
        let mut succ = Vec::new();
        let mut returns = Vec::new();
        for (b, block) in m.blocks.iter().enumerate() {
            let next = match block.term? {
                Term::Br(t) => vec![t],
                Term::CondBr(_, t, f) if t == f => vec![t],
                Term::CondBr(_, t, f) => vec![t, f],
                Term::Ret => {
                    returns.push(b);
                    Vec::new()
                }
            };
            succ.push(next);
        }
        let exit = match returns.as_slice() {
            [] => return None,
            [only] => *only,
            _ => {
                let join = succ.len();
                for block in returns {
                    succ[block].push(join);
                }
                succ.push(Vec::new());
                join
            }
        };
        // The writer omits blocks made unreachable by module optimization.
        let reachable = reachable_from(&succ, 0);
        let mut pred = vec![Vec::new(); succ.len()];
        for (b, next) in succ.iter_mut().enumerate() {
            if !reachable[b] {
                next.clear();
            }
            for &t in next.iter() {
                pred[t].push(b);
            }
        }
        let reaches_exit = reachable_from(&pred, exit);
        if reachable
            .iter()
            .zip(reaches_exit)
            .any(|(&live, exits)| live && !exits)
        {
            return None;
        }
        let dom = Dominators::new(&pred, 0);
        let postdom = Dominators::new(&succ, exit);
        Some(Self {
            succ,
            pred,
            reachable,
            dom,
            postdom,
        })
    }

    fn region(&self, b: usize) -> Vec<usize> {
        let mut region = Vec::new();
        let mut seen = vec![false; self.succ.len()];
        let mut pending = self.succ[b].clone();
        while let Some(t) = pending.pop() {
            // A loop may revisit its own branch before reconvergence.
            if seen[t] || (t != b && self.postdom.has(b, t)) {
                continue;
            }
            seen[t] = true;
            region.push(t);
            pending.extend(self.succ[t].iter().copied());
        }
        region
    }

    fn capped_graph_is_acyclic(&self, caps: &[Option<usize>]) -> bool {
        let mut incoming: Vec<_> = self.pred.iter().map(Vec::len).collect();
        for &header in caps.iter().flatten() {
            incoming[header] -= 1;
        }
        let mut pending: Vec<_> = (0..incoming.len()).filter(|&b| incoming[b] == 0).collect();
        let mut order = vec![0; incoming.len()];
        let mut visited = 0;
        while let Some(b) = pending.pop() {
            order[b] = visited;
            visited += 1;
            for &t in &self.succ[b] {
                if caps[b] == Some(t) {
                    continue;
                }
                incoming[t] -= 1;
                if incoming[t] == 0 {
                    pending.push(t);
                }
            }
        }
        visited == incoming.len()
            && caps
                .iter()
                .enumerate()
                .all(|(b, h)| h.is_none_or(|h| order[h] <= order[b]))
    }
}

fn reachable_from(edges: &[Vec<usize>], root: usize) -> Vec<bool> {
    let mut seen = vec![false; edges.len()];
    let mut pending = vec![root];
    while let Some(b) = pending.pop() {
        if !seen[b] {
            seen[b] = true;
            pending.extend(edges[b].iter().copied());
        }
    }
    seen
}

struct Dominators(Vec<Vec<u64>>);

impl Dominators {
    fn new(pred: &[Vec<usize>], root: usize) -> Self {
        let n = pred.len();
        let full = vec![u64::MAX; n.div_ceil(u64::BITS as usize)];
        let mut dom = vec![full.clone(); n];
        dom[root].fill(0);
        dom[root][root / 64] = 1 << (root % 64);
        let mut changed = true;
        let mut row = full.clone();
        while changed {
            changed = false;
            for (b, incoming) in pred.iter().enumerate().filter(|(b, _)| *b != root) {
                row.copy_from_slice(&full);
                for &p in incoming {
                    for (bits, pred) in row.iter_mut().zip(&dom[p]) {
                        *bits &= pred;
                    }
                }
                row[b / 64] |= 1 << (b % 64);
                if dom[b] != row {
                    std::mem::swap(&mut dom[b], &mut row);
                    changed = true;
                }
            }
        }
        Self(dom)
    }

    fn has(&self, block: usize, dominator: usize) -> bool {
        self.0[block][dominator / 64] & (1 << (dominator % 64)) != 0
    }
}

struct Proof<'a> {
    m: &'a Module,
    cfg: ControlFlow,
    /// Definition's block before constants and values are numbered for DXIL.
    block_of: Vec<usize>,
    /// Per latch, the loop header reached by a bounded backedge.
    caps: Vec<Option<usize>>,
}

impl Module {
    pub(crate) fn certifies_paired_lds(
        &self,
        threads: Option<[u32; 3]>,
        wave: Option<u32>,
    ) -> bool {
        if threads != Some([32, 1, 1]) || wave != Some(32) {
            return false;
        }
        let Some(mut proof) = self.group_proof() else {
            return false;
        };
        proof.bound_loops() && proof.align_meetings() && proof.paired_memory()
    }

    fn group_proof(&self) -> Option<Proof<'_>> {
        let cfg = ControlFlow::new(self)?;
        let mut block_of = vec![usize::MAX; self.insts.len()];
        for (b, block) in self.blocks.iter().enumerate() {
            if !cfg.reachable[b] {
                continue;
            }
            for &i in block.phis.iter().chain(&block.body) {
                block_of[i as usize] = b;
            }
        }
        Some(Proof {
            m: self,
            caps: vec![None; cfg.succ.len()],
            cfg,
            block_of,
        })
    }
}

impl Proof<'_> {
    fn def(&self, v: V) -> Option<(usize, &Op)> {
        let V::Inst(i) = v else { return None };
        let b = self.block_of[i as usize];
        (b != usize::MAX).then_some((b, &self.m.insts[i as usize].op))
    }

    // A natural loop header entered once at zero and incremented on its
    // sole backedge. Returns (header, latch, step).
    fn counter(&self, v: V) -> Option<(usize, usize, V)> {
        let (header, Op::Phi(incoming)) = self.def(v)? else {
            return None;
        };
        let [a, b] = incoming.as_slice() else {
            return None;
        };
        let ((init, entry), (step, latch)) = if self.cfg.dom.has(b.1, header) {
            (*a, *b)
        } else {
            (*b, *a)
        };
        if self.m.int_value(init) != Some(0)
            || self.cfg.dom.has(entry, header)
            || !self.cfg.dom.has(latch, header)
            || self.cfg.pred[header].len() != 2
            || !self.cfg.pred[header].contains(&entry)
            || !self.cfg.pred[header].contains(&latch)
            || self.m.types[self.m.type_of(v).0 as usize] != TypeDef::Int(32)
        {
            return None;
        }
        let (block, Op::Bin(Bin::Add, a, b)) = self.def(step)? else {
            return None;
        };
        let adds_one = (*a == v && self.m.int_value(*b) == Some(1))
            || (*b == v && self.m.int_value(*a) == Some(1));
        (block == latch && adds_one).then_some((header, latch, step))
    }

    // Removing the capped edges must make the graph acyclic. An infinite
    // path would take some cap infinitely often. The earliest such header
    // in the resulting topological order would then never reset its count.
    fn bound_loops(&mut self) -> bool {
        for (b, edges) in self.cfg.succ.iter().enumerate() {
            let mut back = edges.iter().copied().filter(|&t| self.cfg.dom.has(b, t));
            let Some(header) = back.next() else { continue };
            if back.next().is_some() || !self.caps_exit(b, header) {
                return false;
            }
            self.caps[b] = Some(header);
        }
        self.cfg.capped_graph_is_acyclic(&self.caps)
    }

    fn caps_exit(&self, latch: usize, header: usize) -> bool {
        let Some(Term::CondBr(cond, yes, no)) = self.m.blocks[latch].term else {
            return false;
        };
        let Some((_, Op::Bin(bin, a, b))) = self.def(cond) else {
            return false;
        };
        let cmp = match bin {
            Bin::Or if no == header && yes != header => pred::IUGE,
            Bin::And if yes == header && no != header => pred::IULT,
            _ => return false,
        };
        [*a, *b].into_iter().any(|part| {
            let Some((_, Op::Cmp(op, step, limit))) = self.def(part) else {
                return false;
            };
            let Some((_, Op::Bin(Bin::Add, x, y))) = self.def(*step) else {
                return false;
            };
            *op == cmp
                && self.m.int_value(*limit).is_some_and(|n| n as u32 != 0)
                && [*x, *y]
                    .into_iter()
                    .any(|phi| self.counter(phi) == Some((header, latch, *step)))
        })
    }

    fn wait_mode(&self, op: &Op) -> Option<i64> {
        match op {
            Op::Call(f, args) if self.m.funcs[*f as usize].name == "dx.op.barrier" => {
                // An unavailable mode still counts as a possible meeting.
                Some(args.get(1).and_then(|&v| self.m.int_value(v)).unwrap_or(1))
            }
            _ => None,
        }
    }

    fn align_meetings(&self) -> bool {
        let mut checkpoints: Vec<_> = self
            .m
            .blocks
            .iter()
            .enumerate()
            .map(|(b, block)| {
                self.cfg.reachable[b]
                    && block.body.iter().any(|&i| {
                        self.wait_mode(&self.m.insts[i as usize].op)
                            .is_some_and(|mode| mode & 1 != 0)
                    })
            })
            .collect();
        checkpoints.resize(self.cfg.succ.len(), false);
        if !checkpoints.iter().any(|&wait| wait) {
            return false;
        }
        let branches: Vec<_> = self
            .m
            .blocks
            .iter()
            .enumerate()
            .filter_map(|(b, block)| {
                if let Some(Term::CondBr(v, t, f)) = block.term
                    && t != f
                    && self.cfg.reachable[b]
                {
                    Some((b, v, self.cfg.region(b)))
                } else {
                    None
                }
            })
            .collect();

        // Every path through a varying split must reconverge before it
        // reaches a meeting. If a meeting depends on a loop counter,
        // also require both the entry and the latch to be reached together.
        let mut checked = vec![false; checkpoints.len()];
        let mut changed = true;
        while changed {
            changed = false;
            for (b, v, region) in &branches {
                if checked[*b] || !region.iter().any(|&q| checkpoints[q]) {
                    continue;
                }
                if !self.uniform(*v, *b, &mut checkpoints, &mut Default::default()) {
                    return false;
                }
                checked[*b] = true;
                checkpoints[*b] = true;
                changed = true;
            }
        }
        true
    }

    fn constant(&self, c: u32) -> bool {
        match &self.m.consts[c as usize].1 {
            Const::Int(_) | Const::Float(_) => true,
            Const::Aggregate(parts) => parts.iter().all(|&part| self.constant(part)),
            Const::Undef => false,
        }
    }

    fn uniform(&self, v: V, b: usize, checkpoints: &mut [bool], proven: &mut HashSet<V>) -> bool {
        if proven.contains(&v) {
            return true;
        }
        if let Some((at, Op::Call(f, args))) = self.def(v)
            && let [opcode, _] = args.as_slice()
            && matches!(
                (
                    self.m.funcs[*f as usize].name.as_str(),
                    self.m.int_value(*opcode)
                ),
                ("dx.op.waveAnyTrue", Some(113))
                    | ("dx.op.waveAllTrue", Some(114))
                    | ("dx.op.waveReadLaneFirst.i32", Some(118))
            )
        {
            // A wave result is group-uniform only for one full invocation of
            // this collective. The fixed point checks its block as a meeting.
            checkpoints[at] = true;
            proven.insert(v);
            return true;
        }
        let same = &mut |v| self.uniform(v, b, checkpoints, proven);
        let valid = match self.def(v) {
            Some((_, Op::Phi(_))) => self.counter(v).is_some_and(|(header, latch, _)| {
                if self.caps[latch] != Some(header) {
                    return false;
                }
                checkpoints[header] = true;
                checkpoints[latch] = true;
                true
            }),
            Some((
                _,
                Op::Bin(Bin::Add | Bin::Sub | Bin::Mul | Bin::And | Bin::Or | Bin::Xor, a, c),
            ))
            | Some((_, Op::Cmp(32..=41, a, c))) => same(*a) && same(*c),
            Some((_, Op::Bin(Bin::Shl | Bin::LShr | Bin::AShr, a, c))) => {
                self.m.types[self.m.type_of(*a).0 as usize] == TypeDef::Int(32)
                    && self
                        .m
                        .int_value(*c)
                        .is_some_and(|shift| (0..32).contains(&shift))
                    && same(*a)
            }
            Some((_, Op::Extract(a, _))) => same(*a),
            Some((_, Op::Call(f, args))) => {
                let accepted = match self.m.funcs[*f as usize].name.as_str() {
                    "dx.op.groupId.i32"
                    | "dx.op.annotateHandle"
                    | "dx.op.createHandleFromBinding"
                    | "dx.op.createHandleFromHeap" => true,
                    // The view heap caps indices at its null slot with UMin.
                    // Both operands still have to be group-uniform below.
                    "dx.op.binary.i32" => matches!(args.as_slice(), [op, _, limit]
                        if self.m.int_value(*op) == Some(40) && self.m.int_value(*limit).is_some()),
                    "dx.op.cbufferLoadLegacy.i32" => {
                        args.get(1).is_some_and(|&handle| self.cbuffer(handle))
                    }
                    _ => false,
                };
                accepted && args.iter().copied().all(same)
            }
            Some((at, Op::Load(ptr))) if at == b => self.snapshot(v, *ptr, b),
            _ => matches!(v, V::Const(c) if self.constant(c)),
        };
        if valid {
            proven.insert(v);
        }
        valid
    }

    fn cbuffer(&self, handle: V) -> bool {
        let Some((_, Op::Call(f, args))) = self.def(handle) else {
            return false;
        };
        let [_, _, V::Const(props)] = args.as_slice() else {
            return false;
        };
        let Const::Aggregate(fields) = &self.m.consts[*props as usize].1 else {
            return false;
        };
        self.m.funcs[*f as usize].name == "dx.op.annotateHandle"
            && fields
                .first()
                .is_some_and(|&kind| self.m.int_value(V::Const(kind)) == Some(13))
    }

    fn snapshot(&self, v: V, ptr: V, b: usize) -> bool {
        let V::Inst(load) = v else { return false };
        let Some((_, Op::Gep(array, V::Global(g), indices))) = self.def(ptr) else {
            return false;
        };
        let TypeDef::Array(len, elem) = self.m.types[array.0 as usize] else {
            return false;
        };
        let [leading, slot] = indices.as_slice() else {
            return false;
        };
        let Some(slot) = self.m.int_value(*slot) else {
            return false;
        };
        if self.m.globals[*g as usize].space != 3
            || self.m.types[elem.0 as usize] != TypeDef::Int(32)
            || self.m.int_value(*leading) != Some(0)
            || !(0..i64::from(len)).contains(&slot)
        {
            return false;
        }
        let body = &self.m.blocks[b].body;
        let Some(at) = body.iter().position(|&i| i == load) else {
            return false;
        };
        let fence = |&i: &u32| {
            self.wait_mode(&self.m.insts[i as usize].op)
                .is_some_and(|mode| mode & 9 == 9)
        };
        let Some(before) = body[..at].iter().rposition(fence) else {
            return false;
        };
        let Some(after) = body[at + 1..].iter().position(fence) else {
            return false;
        };
        // No other lane can update the dword between the two barriers.
        body[before + 1..at + 1 + after].iter().all(|&i| {
            i == load
                || matches!(
                    self.m.insts[i as usize].op,
                    Op::Gep(..) | Op::Cmp(..) | Op::Bin(..) | Op::Cast(..)
                )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Attr, Module, Term, V, pred};

    fn meeting(m: &mut Module, block: usize) {
        barrier(m, block, 9);
    }

    fn barrier(m: &mut Module, block: usize, mode: u32) -> V {
        let void = m.void();
        let i32t = m.int(32);
        let ty = m.func_ty(void, vec![i32t, i32t]);
        let barrier = m.function("dx.op.barrier", ty, Some(Attr::NoDuplicate), false);
        let op = m.i32(80);
        let mode = m.i32(mode);
        m.call(block, barrier, vec![op, mode])
    }

    #[test]
    fn paired_lds_counts_a_converged_vote_as_group_uniform_only_for_one_wave() {
        for (threads, vote, accepted) in [
            ([32, 1, 1], true, true),
            ([64, 1, 1], true, false),
            ([32, 1, 1], false, false),
        ] {
            let mut m = Module::default();
            let entry = m.new_block();
            let wait = m.new_block();
            let exit = m.new_block();
            let i32t = m.int(32);
            let i1t = m.int(1);
            let lane_ty = m.func_ty(i32t, vec![i32t]);
            let lane = m.function(
                "dx.op.waveGetLaneIndex",
                lane_ty,
                Some(Attr::ReadOnly),
                false,
            );
            let op = m.i32(111);
            let lane = m.call(entry, lane, vec![op]);
            let zero = m.i32(0);
            let varying = m.cmp(entry, pred::EQ, lane, zero);
            let cond = if vote {
                let ty = m.func_ty(i1t, vec![i32t, i1t]);
                let any = m.function("dx.op.waveAnyTrue", ty, Some(Attr::NoUnwind), false);
                let op = m.i32(113);
                m.call(entry, any, vec![op, varying])
            } else {
                varying
            };
            m.terminate(entry, Term::CondBr(cond, wait, exit));
            meeting(&mut m, wait);
            m.terminate(wait, Term::Ret);
            m.terminate(exit, Term::Ret);
            assert_eq!(m.certifies_paired_lds(Some(threads), Some(32)), accepted);
        }
    }

    #[test]
    fn paired_lds_checks_plain_store_writers_and_addresses() {
        for (one_writer, index, accepted) in [(true, 0, true), (true, 4, false), (false, 0, false)]
        {
            let mut m = Module::default();
            let entry = m.new_block();
            let write = m.new_block();
            let exit = m.new_block();
            let i32t = m.int(32);
            let lane_ty = m.func_ty(i32t, vec![i32t]);
            let lane = m.function(
                "dx.op.waveGetLaneIndex",
                lane_ty,
                Some(Attr::ReadOnly),
                false,
            );
            let op = m.i32(111);
            let lane = m.call(entry, lane, vec![op]);
            let zero = m.i32(0);
            let cond = m.cmp(entry, pred::EQ, lane, zero);
            m.terminate(
                entry,
                if one_writer {
                    Term::CondBr(cond, write, exit)
                } else {
                    Term::Br(write)
                },
            );
            let arr = m.array(4, i32t);
            let lds = m.global("lds", arr, 3);
            let index = m.i32(index);
            let ptr = m.element(write, lds, index);
            m.store(write, ptr, lane);
            m.terminate(write, Term::Br(exit));
            meeting(&mut m, exit);
            m.terminate(exit, Term::Ret);
            assert_eq!(m.certifies_paired_lds(Some([32, 1, 1]), Some(32)), accepted);
        }
    }

    #[test]
    fn paired_lds_requires_the_correct_bound_for_a_dynamic_access() {
        use super::super::Rmw;

        for (limit, accepted) in [(Some(4), true), (Some(5), false), (None, false)] {
            let mut m = Module::default();
            let entry = m.new_block();
            let atomic = m.new_block();
            let exit = m.new_block();
            let i32t = m.int(32);
            let id_ty = m.func_ty(i32t, vec![i32t, i32t]);
            let id = m.function("dx.op.groupId.i32", id_ty, Some(Attr::ReadNone), false);
            let op = m.i32(94);
            let x = m.i32(0);
            let index = m.call(entry, id, vec![op, x]);
            if let Some(limit) = limit {
                let limit = m.i32(limit);
                let cond = m.cmp(entry, pred::IULT, index, limit);
                m.terminate(entry, Term::CondBr(cond, atomic, exit));
            } else {
                m.terminate(entry, Term::Br(atomic));
            }
            let arr = m.array(4, i32t);
            let lds = m.global("lds", arr, 3);
            let ptr = m.element(atomic, lds, index);
            let value = m.i32(1);
            m.atomic(atomic, Rmw::Add, ptr, value);
            m.terminate(atomic, Term::Br(exit));
            meeting(&mut m, exit);
            m.terminate(exit, Term::Ret);
            assert_eq!(m.certifies_paired_lds(Some([32, 1, 1]), Some(32)), accepted);
        }
    }
}
